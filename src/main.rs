use clap::Parser;
use std::io::{BufRead, BufReader, Result, Write};
use std::os::unix::net::UnixStream;

use crate::target_action::handle_target;
use crate::utils::get_socket_path;
pub mod args;
pub mod daemon;
pub mod register_action;
pub mod state;
pub mod target_action;
pub mod utils;

fn connect_or_start_daemon(socket_path: &str) -> Result<UnixStream> {
    if let Ok(stream) = UnixStream::connect(socket_path) {
        return Ok(stream);
    }

    let exe = std::env::current_exe()?;
    std::process::Command::new(exe).arg("daemon").spawn()?;

    for _ in 0..40 {
        std::thread::sleep(std::time::Duration::from_millis(50));
        if let Ok(stream) = UnixStream::connect(socket_path) {
            return Ok(stream);
        }
    }

    Err(std::io::Error::new(
        std::io::ErrorKind::NotConnected,
        "Failed to start daemon",
    ))
}

fn main() -> Result<()> {
    if std::env::args().any(|arg| arg == "daemon") {
        return daemon::run_daemon();
    }
    let args = args::Args::parse();
    if let args::Action::Target {
        property,
        spawn,
        as_float,
        animations,
    } = args.action
    {
        handle_target(property, spawn, as_float, animations)?;
        return Ok(());
    }

    let path_buf = get_socket_path()?;
    let Some(socket_path) = path_buf.to_str() else {
        panic!("socket path malformed");
    };
    let mut stream = connect_or_start_daemon(&socket_path)?;
    let request = serde_json::to_string(&args.action)?;
    writeln!(stream, "{}", request)?;

    let mut reader = BufReader::new(&stream);
    let mut response = String::new();
    reader.read_line(&mut response)?;

    print!("{}", response.trim());

    Ok(())
}
