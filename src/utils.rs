use niri_ipc::{Action::MoveWindowToFloating, Request, socket::Socket};

pub fn set_floating(socket: &mut Socket, window_id: u64) {
    let floating_action = MoveWindowToFloating {
        id: (Some(window_id)),
    };
    socket.send(Request::Action(floating_action)).ok();
}
