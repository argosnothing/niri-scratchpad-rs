use niri_ipc::socket::Socket;
use niri_ipc::{Action::MoveWindowToFloating, Action::MoveWindowToTiling, Request};
use std::env::var;
use std::{io::Result, path::PathBuf};

use crate::args::{PropertyKind, ScratchpadOpts};
use crate::register_action::stash;
use crate::state::{Register, State};
use crate::target_action::{get_windows_by_property, stash_window};
use crate::worker::Worker;

pub enum ActionPerformed {
    Summoned,
    Stashed,
}

pub enum Scratchpad {
    Register(Register, Option<ScratchpadOpts>),
    Target(PropertyKind, String, Option<ScratchpadOpts>),
}

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
pub fn stash_scratchpads(
    except: Scratchpad,
    socket: &mut Socket,
    state: &mut State,
    worker: Option<&Worker>,
    workspace_id: u64,
) {
    let registers_to_stash = state.registers.iter().filter(
        |register| !matches!(&except, Scratchpad::Register(i, _) if i.number == register.number),
    );
}
