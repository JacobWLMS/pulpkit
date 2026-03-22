//! Pulpkit — a Rust desktop shell framework.
//!
//! Usage:
//!   pulpkit-core <shell-directory>     Run the shell
//!   pulpkit-core msg <lua-code>        Send a command to the running shell

use std::io::Write;
use std::os::unix::net::UnixStream;
use std::path::PathBuf;

mod dirty;
mod event_loop;
mod events;
mod ipc;
mod popups;
mod runtime;
mod setup;
mod surfaces;
mod theme;
mod timers;
mod watcher;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();

    // pulpkit-core msg <lua-code> — send IPC command
    if args.get(1).map(|s| s.as_str()) == Some("msg") {
        let cmd = args[2..].join(" ");
        if cmd.is_empty() {
            eprintln!("Usage: pulpkit-core msg <lua-code>");
            std::process::exit(1);
        }
        return send_ipc(&cmd);
    }

    // pulpkit-core <shell-dir> — run the shell
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let shell_dir = match args.get(1) {
        Some(dir) => PathBuf::from(dir),
        None => {
            eprintln!("Usage: pulpkit-core <shell-directory>");
            eprintln!("       pulpkit-core msg <lua-code>");
            std::process::exit(1);
        }
    };

    runtime::run(shell_dir)
}

fn send_ipc(cmd: &str) -> anyhow::Result<()> {
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR")
        .unwrap_or_else(|_| "/tmp".into());
    let socket_path = PathBuf::from(runtime_dir).join("pulpkit.sock");

    let mut stream = UnixStream::connect(&socket_path)
        .map_err(|e| anyhow::anyhow!("Failed to connect to pulpkit (is it running?): {e}"))?;

    writeln!(stream, "{}", cmd)?;
    Ok(())
}
