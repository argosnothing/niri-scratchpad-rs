use crate::register_action::RegisterInformation;
use crate::state::{Register, State};
use crate::target_action::handle_target;
use crate::utils::{get_socket_path, set_floating, set_tiling};
use crate::worker::{self, WorkerThread};
use crate::{
    args::{Action, Output},
    register_action,
};
use niri_ipc::socket::Socket;
use niri_ipc::{Request as NiriRequest, Response as NiriResponse};
use std::os::unix::net::UnixStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::{
    io::{BufRead, BufReader, Result, Write},
    os::unix::net::UnixListener,
};

struct FocusedWindowContext {
    window_id: u64,
    title: Option<String>,
    app_id: Option<String>,
    current_workspace_id: u64,
}

enum ActionResponse {
    End,
    Continue,
}

pub fn run_daemon() -> Result<()> {
    #[cfg(debug_assertions)]
    let _ = std::fs::write("/proc/self/comm", "niri-reg-debug");
    let socket_path = get_socket_path()?;
    if socket_path.exists() {
        std::fs::remove_file(&socket_path)?;
    }
    let listener = UnixListener::bind(&socket_path)?;
    let mut state = Arc::new(Mutex::new(State::new()));
    let mut worker: Option<WorkerThread> = None;
    let shutdown = Arc::new(AtomicBool::new(false));

    for stream in listener.incoming() {
        if shutdown.load(Ordering::Relaxed) {
            break;
        }
        match stream {
            Ok(stream) => match handle_client(stream, &mut state, &mut worker, &shutdown) {
                Ok(ActionResponse::End) => break,
                Err(e) => eprintln!("Error handling client: {}", e),
                _ => {}
            },
            Err(e) => eprintln!("Connection error: {}", e),
        }
    }

    Ok(())
}

fn handle_client(
    stream: UnixStream,
    state: &mut Arc<Mutex<State>>,
    worker: &mut Option<WorkerThread>,
    shutdown: &Arc<AtomicBool>,
) -> Result<ActionResponse> {
    let mut reader = BufReader::new(&stream);
    let mut line = String::new();
    reader.read_line(&mut line)?;

    let action: Action = serde_json::from_str(&line)?;
    let mut socket = Socket::connect()?;

    let response = match action {
        Action::Daemon => return Ok(ActionResponse::Continue),
        Action::Create {
            register_number,
            output,
            as_float,
            animations,
        } => {
            let (
                Ok(NiriResponse::FocusedWindow(focused_window)),
                Ok(NiriResponse::Workspaces(workspaces)),
            ) = (
                socket.send(NiriRequest::FocusedWindow)?,
                socket.send(NiriRequest::Workspaces)?,
            )
            else {
                return Ok(ActionResponse::Continue);
            };
            let Some(current_workspace) = workspaces.iter().find(|workspace| workspace.is_focused)
            else {
                return Ok(ActionResponse::Continue);
            };

            let mut state_lock = state.lock().unwrap();
            let create_response = match focused_window {
                Some(window) => {
                    let result = handle_focused_window(
                        &mut socket,
                        &mut state_lock,
                        register_number,
                        FocusedWindowContext {
                            window_id: window.id,
                            title: window.title,
                            app_id: window.app_id,
                            current_workspace_id: current_workspace.id,
                        },
                        output,
                        as_float,
                        animations,
                    );
                    result.unwrap_or_default()
                }
                None => {
                    handle_no_focused_window(&mut socket, &mut state_lock, register_number);
                    String::new()
                }
            };
            if worker.is_none() {
                *worker = Some(worker::spawn(state.clone(), shutdown.clone()));
            }

            create_response
        }
        Action::Delete {
            register_number,
            output,
        } => {
            if output.is_some() {
                String::new()
            } else {
                let mut state_lock = state.lock().unwrap();
                if state_lock.get_register_by_number(register_number).is_some() {
                    let Ok(_) = register_action::summon(
                        &mut socket,
                        &state_lock,
                        RegisterInformation::Id(register_number),
                    ) else {
                        return Ok(ActionResponse::Continue);
                    };
                    state_lock.delete_register(register_number);
                    if state_lock.registers.is_empty() {
                        if let Some(worker) = worker.take() {
                            worker.stop();
                        }
                        return Ok(ActionResponse::End);
                    }
                }
                String::new()
            }
        }
        Action::Get {
            register_number,
            output,
        } => {
            let mut state_lock = state.lock().unwrap();
            sync_state(&mut socket, &mut state_lock);
            let Some(register) = state_lock.get_register_by_number(register_number) else {
                return Ok(ActionResponse::Continue);
            };
            match output {
                Output::Title => register.title.unwrap_or_default(),
                Output::AppId => register.app_id.unwrap_or_default(),
            }
        }
        Action::Sync => {
            let mut state_lock = state.lock().unwrap();
            sync_state(&mut socket, &mut state_lock);
            String::new()
        }
        Action::Target {
            property,
            spawn,
            as_float,
            animations,
        } => {
            let _ = handle_target(property, spawn, as_float, animations);
            return Ok(ActionResponse::Continue);
        }
    };

    write_response(&stream, &response)?;
    Ok(ActionResponse::Continue)
}

fn write_response(stream: &UnixStream, response: &str) -> Result<()> {
    let mut writer = stream;
    writeln!(writer, "{}", response)?;
    Ok(())
}

fn handle_focused_window(
    socket: &mut Socket,
    state: &mut State,
    register_number: i32,
    context: FocusedWindowContext,
    output: Option<Output>,
    as_float: bool,
    animations: bool,
) -> Option<String> {
    match state.get_register_by_number(register_number) {
        Some(register) => {
            let Ok(Ok(NiriResponse::Windows(windows))) = socket.send(NiriRequest::Windows) else {
                return None;
            };
            let register_window = windows.iter().find(|w| w.id == register.window_id)?;

            let output_value = match output {
                Some(Output::Title) => register_window.title.clone(),
                Some(Output::AppId) => register_window.app_id.clone(),
                None => None,
            };

            state.update_register(Register {
                number: register_number,
                title: register_window.title.clone(),
                app_id: register_window.app_id.clone(),
                window_id: register_window.id,
            });

            let Some(workspace_id) = register_window.workspace_id else {
                return output_value;
            };

            if workspace_id == context.current_workspace_id {
                if animations && register_window.is_floating {
                    set_tiling(socket, register_window.id);
                }
                register_action::stash(socket, state, Some(register.number));
            } else {
                register_action::summon(socket, state, RegisterInformation::Register(&register))
                    .ok();

                if as_float && animations {
                    set_floating(socket, register_window.id);
                }
            }

            output_value
        }
        None => {
            state.registers.push(Register {
                title: context.title,
                app_id: context.app_id,
                window_id: context.window_id,
                number: register_number,
            });
            if as_float {
                set_floating(socket, context.window_id);
            }
            None
        }
    }
}

fn handle_no_focused_window(socket: &mut Socket, state: &State, register_number: i32) {
    let Some(register) = state.registers.iter().find(|r| r.number == register_number) else {
        return;
    };

    register_action::summon(socket, state, RegisterInformation::Register(register)).ok();
}

fn sync_state(socket: &mut Socket, state: &mut State) {
    let tracked_registers = state.get_tracked_registers();
    let Ok(register_statuses) = register_action::get_all_register_status(socket, tracked_registers)
    else {
        return;
    };
    state.syncronize_registers(register_statuses).ok();
}
