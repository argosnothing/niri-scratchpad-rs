use niri_ipc::socket::Socket;
use niri_ipc::{Action::MoveWindowToFloating, Action::MoveWindowToTiling, Request};
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

pub fn get_socket_path() -> Result<PathBuf> {
    let runtime_dir = var("XDG_RUNTIME_DIR").map_err(|_| {
        std::io::Error::new(std::io::ErrorKind::NotFound, "XDG_RUNTIME_DIR not set")
    })?;
    Ok(PathBuf::from(runtime_dir).join(format!(
        "niri-register{}.sock",
        if cfg!(debug_assertions) { "-debug" } else { "" }
    )))
}
