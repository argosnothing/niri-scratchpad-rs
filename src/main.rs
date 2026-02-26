use clap::Parser;
use std::env::var;
use std::io::{BufRead, BufReader, Result, Write};
use std::os::unix::net::UnixStream;

use crate::target_action::handle_target;
pub mod args;
pub mod daemon;
pub mod register_action;
pub mod state;
pub mod target_action;

fn main() -> Result<()> {
    if std::env::args().any(|arg| arg == "daemon") {
        return daemon::run_daemon();
    }
    let args = args::Args::parse();
    if let args::Action::Target { property, spawn } = args.action {
        handle_target(property, spawn)?;
        return Ok(());
    }
    let runtime_dir = var("XDG_RUNTIME_DIR").map_err(|_| {
        std::io::Error::new(std::io::ErrorKind::NotFound, "XDG_RUNTIME_DIR not set")
    })?;
    let socket_path = format!("{}/niri-register.sock", runtime_dir);
    let mut stream = UnixStream::connect(&socket_path)
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::NotConnected, "Daemon not running"))?;
    let request = serde_json::to_string(&args.action)?;
    writeln!(stream, "{}", request)?;

    let mut reader = BufReader::new(&stream);
    let mut response = String::new();
    reader.read_line(&mut response)?;

    print!("{}", response.trim());

    Ok(())
}
