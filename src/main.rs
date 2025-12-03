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

    let Some(scratchpad_number) = args.next().and_then(|s| s.parse::<i32>().ok()) else {
        eprintln!("No Arg?");
        return Ok(());
    };

    let Some(current_workspace) = workspaces.iter().find(|workspace| workspace.is_focused) else {
        return Ok(());
    };

    let mut state = match state_file {
        Ok(state) => state,
        Err(err) => {
            eprintln!("{}", err);
            return Ok(());
        }
    };

    match focused_window {
        Some(window) => {
            handle_focused_window(&mut socket, state, scratchpad_number, window.id, current_workspace.id)?;
        }
        None => {
            handle_no_focused_window(&mut socket, &state, scratchpad_number)?;
        }
    }

    Ok(())
}

fn handle_focused_window(
    socket: &mut Socket,
    mut state: State,
    scratchpad_number: i32,
    window_id: u64,
    current_workspace_id: u64,
) -> std::io::Result<()> {
    match state.add_scratchpad(scratchpad_number, window_id, None) {
        Added => {
            state.update()?;
        }
        AlreadyExists(scratchpad) => {
            let Ok(Response::Windows(windows)) = socket.send(Request::Windows)? else {
                return Ok(());
            };

            let Some(scratchpad_window) = windows.iter().find(|w| w.id == scratchpad.id) else {
                return Ok(());
            };

            let Some(workspace_id) = scratchpad_window.workspace_id else {
                return Ok(());
            };

            if workspace_id == current_workspace_id {
                ipc::stash(socket, &state)?;
            } else {
                ipc::summon(socket, &scratchpad)?;
            }
        }
    }
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

    ipc::summon(socket, scratchpad)?;
    Ok(())
}
