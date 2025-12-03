use niri_ipc::socket::Socket;
use niri_ipc::{Action::Spawn, Request, Response};
use state::State;

use crate::state::AddResult::{Added, AlreadyExists};
pub mod ipc;
pub mod state;
fn main() -> std::io::Result<()> {
    let mut args = std::env::args().skip(1);
    let state_file = State::new();
    let mut socket = Socket::connect()?;

    let Ok(Response::FocusedWindow(focused_window)) = socket.send(Request::FocusedWindow)? else {
        return Ok(());
    };
    let Ok(Response::Workspaces(workspaces)) = socket.send(Request::Workspaces)? else {
        return Ok(());
    };

    if let Some(scratchpad_number) = args.next().and_then(|s| s.parse::<i32>().ok()) {
        let Some(current_workspace) = workspaces.iter().find(|workspace| workspace.is_focused)
        else {
            return Ok(());
        };
        match state_file {
            Ok(mut state) => match focused_window {
                Some(window) => match state.add_scratchpad(scratchpad_number, window.id, None) {
                    Added => {
                        state.update()?;
                    }
                    AlreadyExists(scratchpad) => {
                        if let Ok(Response::Windows(windows)) = socket.send(Request::Windows)? {
                            if let Some(scratchpad_window) =
                                windows.iter().find(|window| window.id == scratchpad.id)
                            {
                                if let Some(workspace_id) = scratchpad_window.workspace_id {
                                    if workspace_id == current_workspace.id {
                                        ipc::stash(&mut socket, &state)?;
                                    } else {
                                        ipc::summon(&mut socket, &scratchpad)?;
                                    }
                                };
                            };
                        };
                    }
                },
                None => {
                    if let Some(scratchpad) = state
                        .scratchpads
                        .iter()
                        .find(|scratchpad| scratchpad.scratchpad_number == scratchpad_number)
                    {
                        ipc::summon(&mut socket, scratchpad)?;
                        return Ok(());
                    };
                }
            },
            Err(err) => eprintln!("{}", err),
        }
    } else {
        eprintln!("No Arg?");
    }

    Ok(())
}
