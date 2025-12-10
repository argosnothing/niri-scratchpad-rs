use std::io::Result;

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

pub enum ScratchpadInformation<'a> {
    Id(i32),
    Scratchpad(&'a Scratchpad),
}

pub fn summon(
    socket: &mut Socket,
    state: &State,
    scratchpad_info: ScratchpadInformation,
) -> Result<()> {
    let (focused_output, focused_window, workspaces) = match (
        socket.send(Request::FocusedOutput)?,
        socket.send(Request::FocusedWindow)?,
        socket.send(Request::Workspaces)?,
    ) {
        (
            Ok(Response::FocusedOutput(Some(focused_output))),
            Ok(Response::FocusedWindow(focused_window)),
            Ok(Response::Workspaces(workspaces)),
        ) => (focused_output, focused_window, workspaces),
        _ => return Ok(()),
    };
    let scratchpad: &Scratchpad;
    match scratchpad_info {
        ScratchpadInformation::Id(id) => {
            if let Some(scratch) = state.get_scratchpad_ref_by_number(id) {
                scratchpad = scratch;
            } else {
                return Ok(());
            }
        }
        ScratchpadInformation::Scratchpad(scratch) => scratchpad = scratch,
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

pub fn get_all_scratchpad_status(
    socket: &mut Socket,
    scratchpads: Vec<&Scratchpad>,
) -> Result<Vec<ScratchpadUpdate>> {
    let mut scratchpad_state: Vec<ScratchpadUpdate> = Vec::new();
    let Ok(Response::Windows(windows)) = socket.send(Request::Windows)? else {
        return Ok(scratchpad_state); //return an empty map
    };
    if let Some(orphaned_scratchpad) = scratchpads
        .iter()
        .find(|scratchpad| !windows.iter().any(|window| window.id == scratchpad.id))
    {
        scratchpad_state.push(ScratchpadUpdate::Delete(
            orphaned_scratchpad.scratchpad_number,
        ))
    };
    for window in windows {
        if let Some(scratchpad) = scratchpads
            .iter()
            .find(|scratchpad| scratchpad.id == window.id)
        {
            scratchpad_state.push(ScratchpadUpdate::Update(Scratchpad {
                id: window.id,
                title: window.title.clone(),
                app_id: window.app_id.clone(),
                ..**scratchpad
            }));
        };
    }
    Ok(scratchpad_state)
}
