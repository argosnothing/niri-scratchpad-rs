use std::{
    env,
    io::{self, BufRead, BufReader, Write},
    os::unix::net::UnixStream,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicI32, Ordering},
    },
    thread,
    time::Duration,
};

use niri_ipc::Event;

use niri_ipc::socket::Socket;

use crate::{
    register_action::{self, RegisterInformation},
    state::State,
    target_action,
    utils::get_socket_path,
};

use crate::utils::Scratchpad;

impl PartialEq for Scratchpad {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Scratchpad::Register(a, _), Scratchpad::Register(b, _)) => a.number == b.number,
            (Scratchpad::Target(a, av, _), Scratchpad::Target(b, bv, _)) => a == b && av == bv,
            _ => false,
        }
    }
}

/// Worker that interacts with an optional thread that listens to niri events
pub struct Worker {
    pub state: Arc<Mutex<State>>,
    pub shutdown_signal: Arc<AtomicBool>,
    pub scratchpads_following: Arc<Mutex<Vec<Scratchpad>>>,
    pub delete_watcher_count: Arc<AtomicI32>,
    thread: Mutex<Option<WorkerThread>>,
}

struct WorkerThread {
    handle: thread::JoinHandle<()>,
    stop: Arc<AtomicBool>,
}

impl WorkerThread {
    fn stop(&self) {
        self.stop.store(true, Ordering::Relaxed);
    }

    fn is_finished(&self) -> bool {
        self.handle.is_finished()
    }
}

impl Worker {
    pub fn new(state: Arc<Mutex<State>>, shutdown_signal: Arc<AtomicBool>) -> Self {
        Self {
            state,
            shutdown_signal,
            scratchpads_following: Arc::new(Mutex::new(Vec::new())),
            delete_watcher_count: Arc::new(AtomicI32::new(0)),
            thread: Mutex::new(None),
        }
    }

    pub fn should_thread_run(&self) -> bool {
        self.delete_watcher_count.load(Ordering::Relaxed) > 0
            || !self.scratchpads_following.lock().unwrap().is_empty()
    }

    pub fn ensure_thread_running(&self) {
        let mut thread_slot = self.thread.lock().unwrap();

        if let Some(ref wt) = *thread_slot {
            if wt.is_finished() {
                *thread_slot = None;
            }
        }

        if thread_slot.is_some() || !self.should_thread_run() {
            return;
        }

        let stop = Arc::new(AtomicBool::new(false));
        let state = self.state.clone();
        let shutdown = self.shutdown_signal.clone();
        let delete_watcher_count = self.delete_watcher_count.clone();
        let scratchpads_following = self.scratchpads_following.clone();
        let stop_clone = stop.clone();

        let handle = thread::spawn(move || {
            let _ = listen_to_events(
                state,
                stop_clone,
                shutdown,
                delete_watcher_count,
                scratchpads_following,
            );
        });

        *thread_slot = Some(WorkerThread { handle, stop });
    }

    pub fn stop_thread(&self) {
        let mut thread_slot = self.thread.lock().unwrap();
        if let Some(wt) = thread_slot.take() {
            wt.stop();
        }
    }

    pub fn reconcile(&self) {
        if self.should_thread_run() {
            self.ensure_thread_running();
        } else {
            self.stop_thread();
        }
    }

    pub fn add_scratchpad(&self, scratchpad: Scratchpad) {
        self.scratchpads_following.lock().unwrap().push(scratchpad);
        self.reconcile();
    }

    pub fn remove_scratchpad(&self, scratchpad: Scratchpad) {
        self.scratchpads_following
            .lock()
            .unwrap()
            .retain(|s| s != &scratchpad);
        self.reconcile();
    }

    pub fn increment_delete_watchers(&self) {
        self.delete_watcher_count.fetch_add(1, Ordering::Relaxed);
        self.reconcile();
    }

    pub fn decrement_delete_watchers(&self) {
        self.delete_watcher_count
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |v| {
                if v > 0 { Some(v - 1) } else { None }
            })
            .ok();
        self.reconcile();
    }
}

fn listen_to_events(
    state: Arc<Mutex<State>>,
    stop: Arc<AtomicBool>,
    shutdown: Arc<AtomicBool>,
    delete_watcher_count: Arc<AtomicI32>,
    scratchpads_following: Arc<Mutex<Vec<Scratchpad>>>,
) -> io::Result<()> {
    let socket_path = env::var_os(niri_ipc::socket::SOCKET_PATH_ENV)
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "NIRI_SOCKET is not set"))?;

    let stream = UnixStream::connect(socket_path)?;
    stream.set_read_timeout(Some(Duration::from_secs(2)))?;

    let request = serde_json::to_string(&niri_ipc::Request::EventStream).unwrap();
    let mut writer = &stream;
    writer.write_all(request.as_bytes())?;
    writer.write_all(b"\n")?;

    let mut reader = BufReader::new(&stream);
    let mut buf = String::new();
    reader.read_line(&mut buf)?;

    loop {
        if stop.load(Ordering::Relaxed) {
            break;
        }

        buf.clear();
        match reader.read_line(&mut buf) {
            Ok(0) => break,
            Ok(_) => {}
            Err(e)
                if e.kind() == io::ErrorKind::WouldBlock || e.kind() == io::ErrorKind::TimedOut =>
            {
                continue;
            }
            Err(e) if e.kind() == io::ErrorKind::InvalidData => continue,
            Err(_) => break,
        }

        let event: Event = match serde_json::from_str(&buf) {
            Ok(e) => e,
            Err(_) => continue,
        };

        match event {
            Event::WindowClosed { id } => {
                let mut following = scratchpads_following.lock().unwrap();
                let has_targets = following
                    .iter()
                    .any(|s| matches!(s, Scratchpad::Target(..)));
                following.retain(|s| match s {
                    Scratchpad::Register(r, o) => r.window_id != id,
                    Scratchpad::Target(..) => true,
                });
                if has_targets {
                    if let Ok(mut socket) = Socket::connect() {
                        if let Ok(Ok(niri_ipc::Response::Windows(windows))) =
                            socket.send(niri_ipc::Request::Windows)
                        {
                            following.retain(|s| match s {
                                Scratchpad::Register(_, _) => true,
                                Scratchpad::Target(property, value, options) => {
                                    windows.iter().any(|w| {
                                        target_action::match_window_by_property(w, property, value)
                                    })
                                }
                            });
                        }
                    }
                }
                drop(following);

                if delete_watcher_count.load(Ordering::Relaxed) > 0 {
                    let mut state_lock = state.lock().unwrap();
                    let before = state_lock.registers.len();
                    state_lock.registers.retain(|r| r.window_id != id);
                    let removed = before - state_lock.registers.len();

                    if removed > 0 {
                        for _ in 0..removed {
                            delete_watcher_count
                                .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |v| {
                                    if v > 0 { Some(v - 1) } else { None }
                                })
                                .ok();
                        }
                    }
                }

                if delete_watcher_count.load(Ordering::Relaxed) == 0
                    && scratchpads_following.lock().unwrap().is_empty()
                {
                    let registers_empty = state.lock().unwrap().registers.is_empty();
                    if registers_empty {
                        shutdown.store(true, Ordering::Relaxed);
                        if let Ok(socket_path) = get_socket_path() {
                            let _ = UnixStream::connect(socket_path);
                        }
                        break;
                    }
                }
            }
            Event::WorkspaceActivated { id, focused: _ } => {
                let following_scratchpads = scratchpads_following.lock().unwrap();
                if !following_scratchpads.is_empty() {
                    if let Ok(mut socket) = Socket::connect() {
                        for scratchpad in following_scratchpads.iter() {
                            match scratchpad {
                                Scratchpad::Register(register) => {
                                    let state_lock = state.lock().unwrap();
                                    let _ = register_action::summon(
                                        &mut socket,
                                        &state_lock,
                                        RegisterInformation::Register(register),
                                    );
                                }
                                Scratchpad::Target(property, value) => {
                                    let info = target_action::get_windows_by_property(
                                        &mut socket,
                                        property,
                                        value,
                                        0,
                                    );
                                    for window in &info.windows {
                                        let _ =
                                            target_action::summon_window(&mut socket, window, id);
                                    }
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    Ok(())
}
