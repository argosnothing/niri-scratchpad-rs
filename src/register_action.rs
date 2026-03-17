use std::io::Result;

use crate::state::{Register, RegisterUpdate, State};
use niri_ipc::{
    Action::{FocusWindow, MoveWindowToMonitor, MoveWindowToWorkspace, UnsetWorkspaceName},
    Request, Response, WorkspaceReferenceArg,
    socket::Socket,
};

pub fn stash(socket: &mut Socket, state: &State, register_number: Option<i32>) {
    let Ok(Ok(Response::Windows(windows))) = socket.send(Request::Windows) else {
        return;
    };

    let Some(stash_workspace_id) = crate::utils::get_or_create_stash_workspace(socket) else {
        return;
    };
    for window in windows.iter().filter(|window| match register_number {
        Some(register_num) => state
            .registers
            .iter()
            .any(|register| register.number == register_num && register.window_id == window.id),
        None => state
            .registers
            .iter()
            .any(|register| register.window_id == window.id),
    }) {
        let move_action = MoveWindowToWorkspace {
            window_id: Some(window.id),
            reference: niri_ipc::WorkspaceReferenceArg::Id(stash_workspace_id),
            focus: false,
        };
        let _ = socket.send(Request::Action(move_action));
    }
}

pub enum RegisterInformation<'a> {
    Id(i32),
    Register(&'a Register),
}

pub fn clean_status(socket: &mut Socket) {
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
        let _ = socket.send(Request::Action(UnsetWorkspaceName {
            reference: Some(WorkspaceReferenceArg::Id(stash.id)),
        }));
    }
}

pub fn summon(
    socket: &mut Socket,
    state: &State,
    register_info: RegisterInformation,
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
    let found_register: &Register;
    match register_info {
        RegisterInformation::Id(id) => {
            if let Some(register) = state.get_register_ref_by_number(id) {
                found_register = register;
            } else {
                return Ok(());
            }
        }
        RegisterInformation::Register(register) => found_register = register,
    };

    if let Some(focused_window) = focused_window {
        if focused_window.id == found_register.window_id {
            return Ok(());
        }
    };
    let move_action = MoveWindowToMonitor {
        id: Some(found_register.window_id),
        output: focused_output.name,
    };
    let _ = socket.send(Request::Action(move_action));
    let Some(focused_workspace) = workspaces.iter().find(|workspace| workspace.is_focused) else {
        return Ok(());
    };
    let move_action = MoveWindowToWorkspace {
        window_id: Some(found_register.window_id),
        reference: niri_ipc::WorkspaceReferenceArg::Id(focused_workspace.id),
        focus: (true),
    };
    let _ = socket.send(Request::Action(move_action));
    let focus_action = FocusWindow {
        id: (found_register.window_id),
    };
    let _ = socket.send(Request::Action(focus_action));

    clean_status(socket);
    Ok(())
}

pub fn get_all_register_status(
    socket: &mut Socket,
    registers: Vec<&Register>,
) -> Result<Vec<RegisterUpdate>> {
    let mut register_state: Vec<RegisterUpdate> = Vec::new();
    let Ok(Response::Windows(windows)) = socket.send(Request::Windows)? else {
        return Ok(register_state);
    };
    if let Some(orphaned_register) = registers
        .iter()
        .find(|register| !windows.iter().any(|window| window.id == register.window_id))
    {
        register_state.push(RegisterUpdate::Delete(orphaned_register.number))
    };
    for window in windows {
        if let Some(register) = registers
            .iter()
            .find(|register| register.window_id == window.id)
        {
            register_state.push(RegisterUpdate::Update(Register {
                window_id: window.id,
                title: window.title.clone(),
                app_id: window.app_id.clone(),
                ..**register
            }));
        };
    }
    Ok(register_state)
}
