use crate::register_action::{RegisterInformation, RegisterStatus};
use crate::state::{Register, State};
use crate::target_action::handle_target;
use crate::utils::{set_floating, set_tiling};
use crate::{
    args::{Action, Output},
    register_action,
};
use niri_ipc::socket::Socket;
use niri_ipc::{Request as NiriRequest, Response as NiriResponse};
use std::os::unix::net::UnixStream;
use std::{
    env::var,
    io::{BufRead, BufReader, Result, Write},
    os::unix::net::UnixListener,
    path::PathBuf,
};

struct RegisterWithStatus {
    status: RegisterStatus,
    register: Register,
}

struct FocusedWindowContext {
    window_id: u64,
    title: Option<String>,
    app_id: Option<String>,
    current_workspace_id: u64,
}

pub fn run_daemon() -> Result<()> {
    let socket_path = get_socket_path()?;
    if socket_path.exists() {
        std::fs::remove_file(&socket_path)?;
    }
    let listener = UnixListener::bind(&socket_path)?;
    let mut state = State::new();

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                if let Err(e) = handle_client(stream, &mut state) {
                    eprintln!("Error handling client: {}", e);
                }
            }
            Err(e) => eprintln!("Connection error: {}", e),
        }
    }

    Ok(())
}

fn handle_client(stream: UnixStream, state: &mut State) -> Result<()> {
    let mut reader = BufReader::new(&stream);
    let mut line = String::new();
    reader.read_line(&mut line)?;

    let action: Action = serde_json::from_str(&line)?;
    let mut socket = Socket::connect()?;

    let response = match action {
        Action::Daemon => return Ok(()),
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
                return Ok(());
            };
            let Some(current_workspace) = workspaces.iter().find(|workspace| workspace.is_focused)
            else {
                return write_response(&stream, "");
            };

            match focused_window {
                Some(window) => {
                    let result = handle_focused_window(
                        &mut socket,
                        state,
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
                    handle_no_focused_window(&mut socket, state, register_number);
                    String::new()
                }
            }
        }
        Action::Delete {
            register_number,
            output,
        } => {
            if output.is_some() {
                String::new()
            } else {
                if register_check(&mut socket, state, register_number).is_some() {
                    let Ok(_) = register_action::summon(
                        &mut socket,
                        state,
                        RegisterInformation::Id(register_number),
                    ) else {
                        return Ok(());
                    };
                    state.delete_register(register_number);
                }
                String::new()
            }
        }
        Action::Get {
            register_number,
            output,
        } => {
            sync_state(&mut socket, state);
            let Some(register) = state.get_register_by_number(register_number) else {
                return write_response(&stream, "");
            };
            match output {
                Output::Title => register.title.unwrap_or_default(),
                Output::AppId => register.app_id.unwrap_or_default(),
            }
        }
        Action::Sync => {
            sync_state(&mut socket, state);
            String::new()
        }
        Action::Target {
            property,
            spawn,
            as_float,
            animations,
        } => {
            let _ = handle_target(property, spawn, as_float, animations);
            return Ok(());
        }
    };

    write_response(&stream, &response)
}

fn write_response(stream: &UnixStream, response: &str) -> Result<()> {
    let mut writer = stream;
    writeln!(writer, "{}", response)?;
    Ok(())
}

fn register_check(
    socket: &mut Socket,
    state: &State,
    register_number: i32,
) -> Option<RegisterWithStatus> {
    let register = state.get_register_by_number(register_number)?;
    Some(RegisterWithStatus {
        status: register_action::check_status(socket, &register),
        register,
    })
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
    match register_check(socket, state, register_number) {
        Some(register_with_status) => match register_with_status.status {
            RegisterStatus::WindowMapped => {
                let Ok(Ok(NiriResponse::Windows(windows))) = socket.send(NiriRequest::Windows)
                else {
                    return None;
                };
                let register_window = windows
                    .iter()
                    .find(|w| w.id == register_with_status.register.window_id)?;

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
                    register_action::stash(
                        socket,
                        state,
                        Some(register_with_status.register.number),
                    );
                } else {
                    register_action::summon(
                        socket,
                        state,
                        RegisterInformation::Register(&register_with_status.register),
                    )
                    .ok();

                    if as_float && animations {
                        set_floating(socket, register_window.id);
                    }
                }

                output_value
            }
            RegisterStatus::WindowDropped => {
                state.delete_register(register_number);

                let output_value = if let Some(output) = output {
                    match output {
                        Output::Title => context.title.clone(),
                        Output::AppId => context.app_id.clone(),
                    }
                } else {
                    None
                };

                state.registers.push(Register {
                    title: context.title,
                    app_id: context.app_id,
                    window_id: context.window_id,
                    number: register_number,
                });

                if as_float {
                    set_floating(socket, context.window_id);
                }

                output_value
            }
        },
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

fn get_socket_path() -> Result<PathBuf> {
    let runtime_dir = var("XDG_RUNTIME_DIR").map_err(|_| {
        std::io::Error::new(std::io::ErrorKind::NotFound, "XDG_RUNTIME_DIR not set")
    })?;
    Ok(PathBuf::from(runtime_dir).join("niri-register.sock"))
}
