use std::io::Result;

use niri_ipc::{Request, Response, Window, socket::Socket};

use niri_ipc::Action::{
    FocusWindow, MoveWindowToFloating, MoveWindowToMonitor, MoveWindowToWorkspace,
};

use crate::args::Property;

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

pub fn stash_window(socket: &mut Socket, window: &Window, workspace_id: u64) {
    let _ = socket.send(Request::Action(niri_ipc::Action::MoveWindowToWorkspace {
        window_id: Some(window.id),
        reference: niri_ipc::WorkspaceReferenceArg::Id(workspace_id),
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
        reference: niri_ipc::WorkspaceReferenceArg::Id(workspace_id),
        focus: (true),
    };
    let _ = socket.send(Request::Action(move_action));
    let focus_action = FocusWindow { id: (window.id) };
    let _ = socket.send(Request::Action(focus_action));
    Ok(())
}

pub fn spawn(socket: &mut Socket, command: String) {
    let _ = socket.send(Request::Action(niri_ipc::Action::SpawnSh { command }));
}
