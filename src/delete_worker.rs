use std::{
    io,
    os::unix::net::UnixStream,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread,
};

use niri_ipc::{Event, Response, socket::Socket};

use crate::{state::State, utils::get_socket_path};

pub struct DeleteWorkerThread {
    pub handle: thread::JoinHandle<()>,
    pub stop: Arc<AtomicBool>,
}

impl DeleteWorkerThread {
    pub fn stop(&self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

pub fn spawn(state: Arc<Mutex<State>>, shutdown: Arc<AtomicBool>) -> DeleteWorkerThread {
    let stop = Arc::new(AtomicBool::new(false));
    let stop_clone = stop.clone();
    let handle = thread::spawn(move || {
        watch_for_deletion(state, stop_clone, shutdown).unwrap();
    });
    DeleteWorkerThread { handle, stop }
}

pub fn watch_for_deletion(
    state: Arc<Mutex<State>>,
    stop: Arc<AtomicBool>,
    shutdown: Arc<AtomicBool>,
) -> io::Result<()> {
    let mut socket = Socket::connect()?;
    let reply = socket.send(niri_ipc::Request::EventStream)?;
    if matches!(reply, Ok(Response::Handled)) {
        let mut read_event = socket.read_events();
        loop {
            if stop.load(Ordering::Relaxed) {
                break;
            }
            match read_event() {
                Ok(Event::WindowClosed { id }) => {
                    let mut state_lock = state.lock().unwrap();
                    state_lock.registers.retain(|r| r.window_id != id);
                    if state_lock.registers.is_empty() {
                        shutdown.store(true, Ordering::Relaxed);
                        if let Ok(socket_path) = get_socket_path() {
                            let _ = UnixStream::connect(socket_path);
                        }
                        break;
                    }
                }
                Err(e) if e.kind() == io::ErrorKind::InvalidData => {}
                Err(_) => break,
                _ => {}
            }
        }
    };
    Ok(())
}
