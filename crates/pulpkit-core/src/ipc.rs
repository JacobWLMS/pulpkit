//! IPC server — listens for commands on a Unix socket.
//!
//! A background thread accepts connections and reads commands (one per line).
//! Commands are sent to the event loop via a calloop channel, which wakes
//! the loop immediately — no polling delay.
//!
//! Socket path: $XDG_RUNTIME_DIR/pulpkit.sock (or /tmp/pulpkit.sock)

use std::io::{BufRead, BufReader};
use std::os::unix::net::UnixListener;
use std::path::PathBuf;

use calloop::channel::{self, Channel, Sender};

/// Start the IPC server. Returns the calloop channel source and socket path.
///
/// The channel receives command strings that should be executed as Lua code.
/// Insert the channel into the calloop event loop to wake on incoming commands.
pub fn start_ipc_server() -> Option<(Channel<String>, PathBuf)> {
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR")
        .unwrap_or_else(|_| "/tmp".into());
    let socket_path = PathBuf::from(runtime_dir).join("pulpkit.sock");

    // Remove stale socket.
    let _ = std::fs::remove_file(&socket_path);

    let listener = match UnixListener::bind(&socket_path) {
        Ok(l) => l,
        Err(e) => {
            log::warn!("Failed to create IPC socket: {e}");
            return None;
        }
    };

    log::info!("IPC listening on {}", socket_path.display());

    let (sender, channel) = channel::channel::<String>();
    let path_clone = socket_path.clone();

    // Background thread: accept connections and forward commands.
    std::thread::spawn(move || {
        ipc_accept_loop(listener, sender);
        // Cleanup on thread exit.
        let _ = std::fs::remove_file(&path_clone);
    });

    Some((channel, socket_path))
}

fn ipc_accept_loop(listener: UnixListener, sender: Sender<String>) {
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let reader = BufReader::new(stream);
                for line in reader.lines() {
                    match line {
                        Ok(cmd) => {
                            let cmd = cmd.trim().to_string();
                            if !cmd.is_empty() {
                                if sender.send(cmd).is_err() {
                                    return; // channel closed, runtime exiting
                                }
                            }
                        }
                        Err(_) => break,
                    }
                }
            }
            Err(e) => {
                log::debug!("IPC accept error: {e}");
            }
        }
    }
}
