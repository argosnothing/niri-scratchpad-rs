use niri_ipc::{Action::MoveWindowToFloating, Action::MoveWindowToTiling, Request, socket::Socket};

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
