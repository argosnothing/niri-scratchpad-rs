use niri_ipc::socket::Socket;
use niri_ipc::{
    Action::MoveWindowToFloating, Action::MoveWindowToTiling, Request, Response,
    WorkspaceReferenceArg,
};
use std::env::var;
use std::{io::Result, path::PathBuf};

pub fn set_floating(socket: &mut Socket, window_id: u64) {
    let floating_action = MoveWindowToFloating {
        id: (Some(window_id)),
    };
    socket.send(Request::Action(floating_action)).ok();
}

pub fn set_tiling(socket: &mut Socket, window_id: u64) {
    let tiling_action = MoveWindowToTiling {
        id: (Some(window_id)),
    };
    socket.send(Request::Action(tiling_action)).ok();
}

pub fn get_or_create_stash_workspace(socket: &mut Socket) -> Option<u64> {
    let Ok(Ok(Response::Workspaces(workspaces))) = socket.send(Request::Workspaces) else {
        return None;
    };

    if let Some(stash) = workspaces
        .iter()
        .find(|w| w.name.as_deref() == Some(crate::STASH_NAME))
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
        name: crate::STASH_NAME.to_string(),
        workspace: Some(WorkspaceReferenceArg::Id(last.id)),
    }));

    Some(last.id)
}

pub fn cleanup_stash_workspace(socket: &mut Socket) {
    let (windows, workspaces) = match (
        socket.send(Request::Windows),
        socket.send(Request::Workspaces),
    ) {
        (Ok(Ok(Response::Windows(windows))), Ok(Ok(Response::Workspaces(workspaces)))) => {
            (windows, workspaces)
        }
        _ => return,
    };

    let Some(stash) = workspaces
        .iter()
        .find(|w| w.name.as_deref() == Some(crate::STASH_NAME))
    else {
        return;
    };

    let stash_contains_windows = windows.iter().any(|w| w.workspace_id == Some(stash.id));
    if !stash_contains_windows {
        let _ = socket.send(Request::Action(niri_ipc::Action::UnsetWorkspaceName {
            reference: Some(WorkspaceReferenceArg::Id(stash.id)),
        }));
    }
}

pub fn get_socket_path() -> Result<PathBuf> {
    let runtime_dir = var("XDG_RUNTIME_DIR").map_err(|_| {
        std::io::Error::new(std::io::ErrorKind::NotFound, "XDG_RUNTIME_DIR not set")
    })?;
    Ok(PathBuf::from(runtime_dir).join(format!(
        "niri-register{}.sock",
        if cfg!(debug_assertions) { "-debug" } else { "" }
    )))
}
