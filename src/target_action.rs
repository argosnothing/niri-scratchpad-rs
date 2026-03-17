use std::io::Result;

use niri_ipc::{Request, Response, Window, WorkspaceReferenceArg, socket::Socket};

use niri_ipc::Action::{FocusWindow, MoveWindowToMonitor, MoveWindowToWorkspace};

use crate::args::Property;
use crate::register_action::clean_status;
use crate::target_action;
use crate::utils::{set_floating, set_tiling};

pub struct WindowTargetInformation {
    pub windows: Vec<Window>,
    pub found_in_stash: bool,
}

pub fn get_windows_by_property(
    socket: &mut Socket,
    property: &Property,
    workspace_id: u64,
) -> WindowTargetInformation {
    let Ok(Ok(Response::Windows(windows))) = socket.send(Request::Windows) else {
        return WindowTargetInformation {
            windows: vec![],
            found_in_stash: false,
        };
    };
    let mut found_in_stash: bool = false;
    let windows = windows
        .into_iter()
        .filter(|w| {
            if match_window_by_property(w, property) {
                if w.workspace_id.is_some_and(|wid| wid == workspace_id) {
                    found_in_stash = true;
                }
                true
            } else {
                false
            }
        })
        .collect();

    WindowTargetInformation {
        windows,
        found_in_stash,
    }
}

pub fn match_window_by_property(window: &Window, property: &Property) -> bool {
    match property {
        Property::AppId { value } => window
            .app_id
            .as_deref()
            .is_some_and(|wappid| wappid == value),
        Property::Title { value } => window
            .title
            .as_deref()
            .is_some_and(|wtitle| wtitle == value),
    }
}

fn get_or_create_stash_workspace(socket: &mut Socket) -> Option<u64> {
    let Ok(Ok(Response::Workspaces(workspaces))) = socket.send(Request::Workspaces) else {
        return None;
    };

    if let Some(stash) = workspaces
        .iter()
        .find(|w| w.name.as_deref() == Some("stash"))
    {
        return Some(stash.id);
    }

    let target_output = match socket.send(Request::FocusedOutput) {
        Ok(Ok(Response::FocusedOutput(Some(output)))) => Some(output.name),
        _ => None,
    };

    let last = if let Some(ref output_name) = target_output {
        workspaces
            .iter()
            .filter(|w| w.output.as_deref() == Some(output_name.as_str()) && w.name.is_none())
            .max_by_key(|w| w.idx)
    } else {
        workspaces
            .iter()
            .filter(|w| w.name.is_none())
            .max_by_key(|w| w.idx)
    }?;

    let _ = socket.send(Request::Action(niri_ipc::Action::SetWorkspaceName {
        name: "stash".to_string(),
        workspace: Some(WorkspaceReferenceArg::Id(last.id)),
    }));

    Some(last.id)
}

pub fn stash_window(socket: &mut Socket, window: &Window) {
    let Some(stash_id) = get_or_create_stash_workspace(socket) else {
        return;
    };
    let _ = socket.send(Request::Action(niri_ipc::Action::MoveWindowToWorkspace {
        window_id: Some(window.id),
        reference: WorkspaceReferenceArg::Id(stash_id),
        focus: false,
    }));
}

pub fn summon_window(socket: &mut Socket, window: &Window, workspace_id: u64) -> Result<()> {
    let Ok(Response::FocusedOutput(Some(output))) = socket.send(Request::FocusedOutput)? else {
        return Ok(());
    };

    let move_action = MoveWindowToMonitor {
        id: Some(window.id),
        output: output.name,
    };
    let _ = socket.send(Request::Action(move_action));
    let move_action = MoveWindowToWorkspace {
        window_id: Some(window.id),
        reference: WorkspaceReferenceArg::Id(workspace_id),
        focus: true,
    };
    let _ = socket.send(Request::Action(move_action));
    let focus_action = FocusWindow { id: window.id };
    let _ = socket.send(Request::Action(focus_action));

    clean_status(socket);
    Ok(())
}

pub fn handle_target(
    property: Property,
    spawn: Option<String>,
    as_float: bool,
    animations: bool,
) -> Result<()> {
    let mut socket = Socket::connect()?;

    let Ok(Response::Workspaces(workspaces)) = socket.send(Request::Workspaces)? else {
        return Ok(());
    };

    let Some(current_workspace) = workspaces.iter().find(|workspace| workspace.is_focused) else {
        return Ok(());
    };

    let current_workspace_id = current_workspace.id;
    // Find existing stash workspace for the lookup — we don't create it here,
    // only stash_window() creates it on demand when we actually need to stash.
    let stash_workspace_id = workspaces
        .iter()
        .find(|w| w.name.as_deref() == Some("stash"))
        .map(|w| w.id)
        .unwrap_or(0);

    let window_target_information =
        get_windows_by_property(&mut socket, &property, stash_workspace_id);

    if let Some(command) = spawn
        && window_target_information.windows.is_empty()
    {
        target_action::spawn(&mut socket, command);
        return Ok(());
    }

    if !window_target_information.windows.is_empty() {
        // tl;dr if there are ny matching windows found in the stash workspace, we simply move
        // everything up to the focused workspace, regardless if there are matched windows in current workspace
        // otherwise we'll be playing switcheroo if matched windows exist in stash and focused simultaneously
        if window_target_information.found_in_stash {
            for window in window_target_information.windows {
                target_action::summon_window(&mut socket, &window, current_workspace_id)?;
                if as_float {
                    set_floating(&mut socket, window.id);
                }
            }
        } else {
            for window in window_target_information.windows {
                if animations && window.is_floating {
                    set_tiling(&mut socket, window.id);
                }
                target_action::stash_window(&mut socket, &window);
            }
        }
    }

    return Ok(());
}

pub fn spawn(socket: &mut Socket, command: String) {
    let _ = socket.send(Request::Action(niri_ipc::Action::SpawnSh { command }));
}
