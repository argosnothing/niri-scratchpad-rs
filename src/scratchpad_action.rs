use std::{collections::HashMap, io::Result};

pub enum ScratchpadStatus {
    WindowMapped,
    WindowDropped,
}

use crate::state::{Scratchpad, ScratchpadUpdate, State};
use niri_ipc::{
    socket::Socket,
    Action::{FocusWindow, MoveWindowToFloating, MoveWindowToMonitor, MoveWindowToWorkspace},
    Request, Response,
};
// Ensures all scratchpads are stashed
pub fn stash(socket: &mut Socket, state: &State, scratchpad_number: Option<i32>) -> Result<()> {
    let Ok(Response::Windows(windows)) = socket.send(Request::Windows)? else {
        return Ok(());
    };
    let Ok(Response::Workspaces(workspaces)) = socket.send(Request::Workspaces)? else {
        return Ok(());
    };
    let Some(stash_workspace) = workspaces
        .iter()
        .find(|workspace| workspace.name.as_deref() == Some("stash"))
    else {
        return Ok(());
    };
    for window in windows.iter().filter(|window| match scratchpad_number {
        Some(scratch_num) => state.scratchpads.iter().any(|scratchpad| {
            scratchpad.scratchpad_number == scratch_num && scratchpad.id == window.id
        }),
        None => state
            .scratchpads
            .iter()
            .any(|scratchpad| scratchpad.id == window.id),
    }) {
        let move_action = MoveWindowToWorkspace {
            window_id: Some(window.id),
            reference: niri_ipc::WorkspaceReferenceArg::Id(stash_workspace.id),
            focus: false,
        };
        let _ = socket.send(Request::Action(move_action));
    }
    Ok(())
}

pub fn summon(socket: &mut Socket, scratchpad: &Scratchpad) -> Result<()> {
    let Ok(Response::FocusedOutput(Some(focused_output))) = socket.send(Request::FocusedOutput)?
    else {
        return Ok(());
    };
    let Ok(Response::FocusedWindow(focused_window)) = socket.send(Request::FocusedWindow)? else {
        return Ok(());
    };
    let Ok(Response::Workspaces(workspaces)) = socket.send(Request::Workspaces)? else {
        return Ok(());
    };
    if let Some(focused_window) = focused_window {
        if focused_window.id == scratchpad.id {
            return Ok(());
        }
    };
    let move_action = MoveWindowToMonitor {
        id: Some(scratchpad.id),
        output: focused_output.name,
    };
    let _ = socket.send(Request::Action(move_action));
    let Some(focused_workspace) = workspaces.iter().find(|workspace| workspace.is_focused) else {
        return Ok(());
    };
    let move_action = MoveWindowToWorkspace {
        window_id: Some(scratchpad.id),
        reference: niri_ipc::WorkspaceReferenceArg::Id(focused_workspace.id),
        focus: (true),
    };
    let _ = socket.send(Request::Action(move_action));
    let focus_action = FocusWindow {
        id: (scratchpad.id),
    };
    let _ = socket.send(Request::Action(focus_action));
    Ok(())
}

pub fn set_floating(socket: &mut Socket, window_id: u64) -> Result<()> {
    let floating_action = MoveWindowToFloating {
        id: (Some(window_id)),
    };
    let _ = socket.send(Request::Action(floating_action));
    Ok(())
}

pub fn check_status(socket: &mut Socket, scratchpad: &Scratchpad) -> Result<ScratchpadStatus> {
    let Ok(Response::Windows(windows)) = socket.send(Request::Windows)? else {
        return Ok(ScratchpadStatus::WindowDropped);
    };
    match windows.iter().find(|window| scratchpad.id == window.id) {
        Some(_) => Ok(ScratchpadStatus::WindowMapped),
        None => Ok(ScratchpadStatus::WindowDropped),
    }
}

pub fn get_all_scratchpad_status<'a>(
    socket: &mut Socket,
    scratchpads: Vec<&'a Scratchpad>,
) -> Result<Vec<ScratchpadUpdate<'a>>> {
    let mut scratchpad_state: Vec<ScratchpadUpdate> = Vec::new();
    let Ok(Response::Windows(windows)) = socket.send(Request::Windows)? else {
        return Ok(scratchpad_state); //return an empty map
    };
    match scratchpads
        .iter()
        .find(|scratchpad| !windows.iter().any(|window| window.id == scratchpad.id))
    {
        Some(orphaned_scratchpad) => {
            scratchpad_state.push(ScratchpadUpdate::Delete(orphaned_scratchpad))
        }
        None => {}
    };
    for window in windows {
        match scratchpads
            .iter()
            .find(|scratchpad| scratchpad.id == window.id)
        {
            Some(scratchpad) => {
                scratchpad_state.push(ScratchpadUpdate::Update(scratchpad));
            }
            None => {}
        };
    }
    Ok(scratchpad_state)
}
