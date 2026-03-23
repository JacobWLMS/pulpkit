//! IPC socket — accepts Lua commands via Unix domain socket.

use std::io::{BufRead, BufReader};
use std::os::unix::net::UnixListener;
use std::path::PathBuf;

use calloop::channel::Sender;

use crate::SubMessage;

/// Start the IPC server on a background thread. Returns the socket path.
///
/// Commands received on the socket are sent as SubMessages with the given
/// msg_name and the command text as data.
pub fn start_ipc_server(msg_name: String, sender: Sender<SubMessage>) -> Option<PathBuf> {
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR").ok()?;
    let socket_path = PathBuf::from(&runtime_dir).join("pulpkit.sock");

    // Remove stale socket
    let _ = std::fs::remove_file(&socket_path);

    let listener = UnixListener::bind(&socket_path).ok()?;
    listener.set_nonblocking(false).ok()?;

    log::info!("IPC socket: {}", socket_path.display());

    let path = socket_path.clone();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let reader = BufReader::new(stream);
                    let sender = sender.clone();
                    let name = msg_name.clone();
                    std::thread::spawn(move || {
                        for line in reader.lines() {
                            match line {
                                Ok(cmd) if !cmd.trim().is_empty() => {
                                    let _ = sender.send(SubMessage {
                                        msg_type: name.clone(),
                                        data: Some(cmd),
                                    });
                                }
                                _ => break,
                            }
                        }
                    });
                }
                Err(e) => {
                    log::error!("IPC accept error: {e}");
                }
            }
        }
    });

    // Clean up socket on drop (best effort)
    Some(path)
}
