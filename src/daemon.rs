use std::{os::unix::net::UnixListener, io::{BufRead, BufReader, Write, Result}, path::PathBuf, env::var};
use crate::{args::{Action, Output}, scratchpad_action};
use crate::state::{State, Scratchpad};
use crate::scratchpad_action::{ScratchpadStatus, ScratchpadInformation, set_floating};
use niri_ipc::socket::Socket;
use niri_ipc::{Request as NiriRequest, Response as NiriResponse};

struct ScratchpadWithStatus {
    status: ScratchpadStatus,
    scratchpad: Scratchpad,
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

fn handle_client(stream: std::os::unix::net::UnixStream, state: &mut State) -> Result<()> {
    let mut reader = BufReader::new(&stream);
    let mut line = String::new();
    reader.read_line(&mut line)?;
    
    let action: Action = serde_json::from_str(&line)?;
    let mut socket = Socket::connect()?;
    
    let Ok(NiriResponse::FocusedWindow(focused_window)) = socket.send(NiriRequest::FocusedWindow)? else {
        return Ok(());
    };
    let Ok(NiriResponse::Workspaces(workspaces)) = socket.send(NiriRequest::Workspaces)? else {
        return Ok(());
    };
    
    let response = match action {
        Action::Daemon => return Ok(()),
        Action::Create { scratchpad_number, output, as_float } => {
            let Some(current_workspace) = workspaces.iter().find(|workspace| workspace.is_focused) else {
                return write_response(&stream, "");
            };
            
            match focused_window {
                Some(window) => {
                    let result = handle_focused_window(
                        &mut socket,
                        state,
                        scratchpad_number,
                        FocusedWindowContext {
                            window_id: window.id,
                            title: window.title,
                            app_id: window.app_id,
                            current_workspace_id: current_workspace.id,
                        },
                        output,
                        as_float,
                    )?;
                    result.unwrap_or_default()
                }
                None => {
                    handle_no_focused_window(&mut socket, state, scratchpad_number)?;
                    String::new()
                }
            }
        }
        Action::Delete { scratchpad_number, output } => {
            if output.is_some() {
                String::new()
            } else {
                if let Ok(Some(_)) = scratchpad_check(&mut socket, state, scratchpad_number) {
                    let Ok(_) = scratchpad_action::summon(&mut socket, state, ScratchpadInformation::Id(scratchpad_number)) else {
                        return Ok(());
                    };
                    state.delete_scratchpad(scratchpad_number);
                }
                String::new()
            }
        }
        Action::Get { scratchpad_number, output } => {
            sync_state(&mut socket, state)?;
            let Some(scratchpad) = state.get_scratchpad_by_number(scratchpad_number) else {
                return write_response(&stream, "");
            };
            match output {
                Output::Title => scratchpad.title.unwrap_or_default(),
                Output::AppId => scratchpad.app_id.unwrap_or_default(),
            }
        }
        Action::Sync => {
            sync_state(&mut socket, state)?;
            String::new()
        }
    };
    
    write_response(&stream, &response)
}

fn write_response(stream: &std::os::unix::net::UnixStream, response: &str) -> Result<()> {
    let mut writer = stream;
    writeln!(writer, "{}", response)?;
    Ok(())
}

fn scratchpad_check(
    socket: &mut Socket,
    state: &State,
    scratchpad_number: i32,
) -> Result<Option<ScratchpadWithStatus>> {
    let Some(scratchpad) = state.get_scratchpad_by_number(scratchpad_number) else {
        return Ok(None);
    };
    match scratchpad_action::check_status(socket, &scratchpad) {
        Ok(status) => Ok(Some(ScratchpadWithStatus { scratchpad, status })),
        Err(_) => Ok(None),
    }
}

fn handle_focused_window(
    socket: &mut Socket,
    state: &mut State,
    scratchpad_number: i32,
    context: FocusedWindowContext,
    output: Option<Output>,
    as_float: bool,
) -> Result<Option<String>> {
    match scratchpad_check(socket, state, scratchpad_number) {
        Ok(Some(scratchpad_with_status)) => match scratchpad_with_status.status {
            ScratchpadStatus::WindowMapped => {
                let Ok(NiriResponse::Windows(windows)) = socket.send(NiriRequest::Windows)? else {
                    return Ok(None);
                };
                let Some(scratchpad_window) = windows
                    .iter()
                    .find(|w| w.id == scratchpad_with_status.scratchpad.id)
                else {
                    return Ok(None);
                };
                
                let output_value = match output {
                    Some(Output::Title) => scratchpad_window.title.clone(),
                    Some(Output::AppId) => scratchpad_window.app_id.clone(),
                    None => None,
                };
                
                state.update_scratchpad(Scratchpad {
                    scratchpad_number,
                    title: scratchpad_window.title.clone(),
                    app_id: scratchpad_window.app_id.clone(),
                    id: scratchpad_window.id
                });
                
                let Some(workspace_id) = scratchpad_window.workspace_id else {
                    return Ok(output_value);
                };

                if workspace_id == context.current_workspace_id {
                    scratchpad_action::stash(
                        socket,
                        state,
                        Some(scratchpad_with_status.scratchpad.scratchpad_number),
                    )?;
                } else {
                    scratchpad_action::summon(socket, state, ScratchpadInformation::Scratchpad(&scratchpad_with_status.scratchpad))?;
                }
                
                Ok(output_value)
            }
            ScratchpadStatus::WindowDropped => {
                state.delete_scratchpad(scratchpad_number);
                
                let output_value = if let Some(output) = output {
                    match output {
                        Output::Title => context.title.clone(),
                        Output::AppId => context.app_id.clone(),
                    }
                } else {
                    None
                };
                
                state.scratchpads.push(Scratchpad {
                    title: context.title,
                    app_id: context.app_id,
                    id: context.window_id,
                    scratchpad_number
                });
                
                if as_float {
                    set_floating(socket, context.window_id);
                }
                
                Ok(output_value)
            }
        },
        Ok(None) => {
            state.scratchpads.push(Scratchpad {
                title: context.title,
                app_id: context.app_id,
                id: context.window_id,
                scratchpad_number
            });
            if as_float {
                set_floating(socket, context.window_id);
            }
            Ok(None)
        }
        Err(_) => Ok(None),
    }
}

fn handle_no_focused_window(
    socket: &mut Socket,
    state: &State,
    scratchpad_number: i32,
) -> Result<()> {
    let Some(scratchpad) = state
        .scratchpads
        .iter()
        .find(|s| s.scratchpad_number == scratchpad_number)
    else {
        return Ok(());
    };

    scratchpad_action::summon(socket, state, ScratchpadInformation::Scratchpad(scratchpad))?;
    Ok(())
}

fn sync_state(socket: &mut Socket, state: &mut State) -> Result<()> {
    let tracked_scratchpads = state.get_tracked_scratchpads();
    let Ok(scratchpad_statuses) = scratchpad_action::get_all_scratchpad_status(socket, tracked_scratchpads) else {
        return Ok(());
    };
    state.syncronize_scratchpads(scratchpad_statuses)
}

fn get_socket_path() -> Result<PathBuf> {
    let runtime_dir = var("XDG_RUNTIME_DIR")
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::NotFound, "XDG_RUNTIME_DIR not set"))?;
    Ok(PathBuf::from(runtime_dir).join("niri-scratchpad.sock"))
}
