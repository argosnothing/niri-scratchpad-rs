use std::clone;

use crate::args::Output;
use crate::scratchpad_action::ScratchpadStatus;
use crate::state::Scratchpad;
use clap::Parser;
use niri_ipc::socket::Socket;
use niri_ipc::{Request, Response};
use state::State;
pub mod args;
pub mod scratchpad_action;
pub mod state;
fn main() -> std::io::Result<()> {
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
                        window.id,
                        window.title,
                        output,
                        current_workspace.id,
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
            match output {
                Some(Output::Title) => print!(""),
                None => (),
            };
            match scratchpad_check(&mut socket, &state, scratchpad_number) {
                Ok(Some(_)) => {
                    state.delete_scratchpad(scratchpad_number)?;
                }
                Ok(None) => return Ok(()),
                Err(_) => return Ok(()),
            };
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
) -> std::io::Result<Option<ScratchpadWithStatus>> {
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
    window_id: u64,
    title: Option<String>,
    output: Option<Output>,
    current_workspace_id: u64,
) -> std::io::Result<()> {
    match scratchpad_check(socket, &state, scratchpad_number) {
        Ok(Some(scratchpad_with_status)) => match scratchpad_with_status.status {
            ScratchpadStatus::WindowMapped => {
                match output {
                    Some(Output::Title) => {
                        if let Some(title) = &scratchpad_with_status.scratchpad.title {
                            print!("{}", title)
                        };
                    },
                    None => (),
                };
                let Ok(Response::Windows(windows)) = socket.send(Request::Windows)? else {
                    return Ok(());
                };

                let Some(scratchpad_window) = windows
                    .iter()
                    .find(|w| w.id == scratchpad_with_status.scratchpad.id)
                else {
                    return Ok(());
                };

                let Some(workspace_id) = scratchpad_window.workspace_id else {
                    return Ok(());
                };

                if workspace_id == current_workspace_id {
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
                        // The focused window is the new assigned window for this output
                        Output::Title => {
                            if let Some(title) = &title {
                                print!("{}", title);
                            };
                        }
                    };
                };
                state.add_scratchpad(scratchpad_number, window_id, title, None)?;
                state.update()?;
            }
        },
        Ok(None) => {
            state.add_scratchpad(scratchpad_number, window_id, title, None)?;
            state.update()?;
        }
        Err(_) => return Ok(()),
    };
    Ok(())
}

fn handle_no_focused_window(
    socket: &mut Socket,
    state: &State,
    scratchpad_number: i32,
) -> std::io::Result<()> {
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
