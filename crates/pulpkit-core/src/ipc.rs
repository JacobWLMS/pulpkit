//! IPC server — listens for commands on a Unix socket.
//!
//! Commands are sent as newline-terminated strings. The runtime polls the
//! socket each event loop iteration and queues commands for processing.
//!
//! Socket path: $XDG_RUNTIME_DIR/pulpkit.sock (or /tmp/pulpkit.sock)

use std::cell::RefCell;
use std::io::{BufRead, BufReader};
use std::os::unix::net::UnixListener;
use std::path::PathBuf;
use std::rc::Rc;

/// Queued IPC commands, polled by the event loop.
pub type IpcCommands = Rc<RefCell<Vec<String>>>;

pub struct IpcServer {
    listener: UnixListener,
    pub socket_path: PathBuf,
    pub commands: IpcCommands,
}

impl IpcServer {
    /// Create and bind the IPC socket. Removes stale socket if it exists.
    pub fn new() -> Option<Self> {
        let runtime_dir = std::env::var("XDG_RUNTIME_DIR")
            .unwrap_or_else(|_| "/tmp".into());
        let socket_path = PathBuf::from(runtime_dir).join("pulpkit.sock");

        // Remove stale socket.
        let _ = std::fs::remove_file(&socket_path);

        match UnixListener::bind(&socket_path) {
            Ok(listener) => {
                listener.set_nonblocking(true).ok();
                log::info!("IPC listening on {}", socket_path.display());
                Some(IpcServer {
                    listener,
                    socket_path,
                    commands: Rc::new(RefCell::new(Vec::new())),
                })
            }
            Err(e) => {
                log::warn!("Failed to create IPC socket: {e}");
                None
            }
        }
    }

    /// Poll for incoming commands (non-blocking).
    pub fn poll(&self) {
        loop {
            match self.listener.accept() {
                Ok((stream, _)) => {
                    let reader = BufReader::new(stream);
                    for line in reader.lines() {
                        match line {
                            Ok(cmd) => {
                                let cmd = cmd.trim().to_string();
                                if !cmd.is_empty() {
                                    self.commands.borrow_mut().push(cmd);
                                }
                            }
                            Err(_) => break,
                        }
                    }
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(_) => break,
            }
        }
    }
}

impl Drop for IpcServer {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.socket_path);
    }
}
