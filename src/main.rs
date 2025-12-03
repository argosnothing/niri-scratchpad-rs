use std::io::Result;

use crate::args::Output;
use crate::scratchpad_action::{set_floating, ScratchpadStatus};
use crate::state::Scratchpad;
use clap::Parser;
use niri_ipc::socket::Socket;
use niri_ipc::{Request, Response};
use state::State;
pub mod args;
pub mod scratchpad_action;
pub mod state;
fn main() -> Result<()> {
    let state_file = State::new();
    let mut socket = Socket::connect()?;
    let args = args::Args::parse();

    let Ok(Response::FocusedWindow(focused_window)) = socket.send(Request::FocusedWindow)? else {
        return Ok(());
    };
    let Ok(Response::Workspaces(workspaces)) = socket.send(Request::Workspaces)? else {
        return Ok(());
    };

    let mut state = match state_file {
        Ok(state) => state,
        Err(err) => {
            eprintln!("{}", err);
            return Ok(());
        }
    };

    match args.action {
        args::Action::Create {
            scratchpad_number,
            output,
            as_float,
        } => {
            let Some(current_workspace) = workspaces.iter().find(|workspace| workspace.is_focused)
            else {
                return Ok(());
            };

            match focused_window {
                Some(window) => {
                    handle_focused_window(
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
                }
                None => {
                    handle_no_focused_window(&mut socket, &state, scratchpad_number)?;
                }
            }
        }
        args::Action::Delete {
            scratchpad_number,
            output,
        } => {
            if output.is_some() {
                print!("")
            };
            match scratchpad_check(&mut socket, &state, scratchpad_number) {
                Ok(Some(_)) => {
                    state.delete_scratchpad(scratchpad_number)?;
                }
                Ok(None) => return Ok(()),
                Err(_) => return Ok(()),
            };
        }
        args::Action::Get {
            scratchpad_number,
            output,
        } => {
            let Some(scratchpad) = state.get_scratchpad_by_number(scratchpad_number) else {
                return Ok(());
            };
            match output {
                Output::Title => {
                    if let Some(title) = scratchpad.title {
                        print!("{}", title)
                    };
                }
                Output::AppId => {
                    if let Some(app_id) = scratchpad.app_id {
                        print!("{}", app_id)
                    }
                }
            }
        }
    };

    Ok(())
}

// if a scratchpad is no longer mapped to a window delete it from our state
// Is Some scratchpad exists, return if it is mapped to a window or not
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
    mut state: State,
    scratchpad_number: i32,
    context: FocusedWindowContext,
    output: Option<Output>,
    as_float: bool,
) -> Result<()> {
    match scratchpad_check(socket, &state, scratchpad_number) {
        Ok(Some(scratchpad_with_status)) => match scratchpad_with_status.status {
            ScratchpadStatus::WindowMapped => {
                let Ok(Response::Windows(windows)) = socket.send(Request::Windows)? else {
                    return Ok(());
                };

                let Some(scratchpad_window) = windows
                    .iter()
                    .find(|w| w.id == scratchpad_with_status.scratchpad.id)
                else {
                    return Ok(());
                };
                match output {
                    Some(Output::Title) => {
                        if let Some(title) = &scratchpad_window.title {
                            print!("{}", title)
                        };
                    }
                    Some(Output::AppId) => {
                        if let Some(app_id) = &scratchpad_window.app_id {
                            print!("{}", app_id)
                        };
                    }
                    None => (),
                };
                state.update_scratchpad(Scratchpad {
                    scratchpad_number,
                    title: scratchpad_window.title.clone(),
                    app_id: scratchpad_window.app_id.clone(),
                    id: scratchpad_window.id,
                    command: None,
                })?;
                let Some(workspace_id) = scratchpad_window.workspace_id else {
                    return Ok(());
                };

                if workspace_id == context.current_workspace_id {
                    scratchpad_action::stash(
                        socket,
                        &state,
                        Some(scratchpad_with_status.scratchpad.scratchpad_number),
                    )?;
                } else {
                    scratchpad_action::summon(socket, &scratchpad_with_status.scratchpad)?;
                }
            }
            ScratchpadStatus::WindowDropped => {
                state.delete_scratchpad(scratchpad_number)?;
                if let Some(output) = output {
                    match output {
                        Output::Title => {
                            if let Some(title) = &context.title {
                                print!("{}", title);
                            };
                        }
                        Output::AppId => {
                            if let Some(app_id) = &context.app_id {
                                print!("{}", app_id);
                            };
                        }
                    };
                };
                state.add_scratchpad(
                    scratchpad_number,
                    context.window_id,
                    context.title,
                    context.app_id,
                    None,
                )?;
                state.update()?;
            }
        },
        Ok(None) => {
            state.add_scratchpad(
                scratchpad_number,
                context.window_id,
                context.title,
                context.app_id,
                None,
            )?;
            state.update()?;
            if as_float {
                set_floating(socket, context.window_id)?;
            }
        }
        Err(_) => return Ok(()),
    };
    Ok(())
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

    scratchpad_action::summon(socket, scratchpad)?;
    Ok(())
}

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
