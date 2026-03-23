//! Exec subscription — runs a command once and returns the full output.

use std::process::{Command, Stdio};

use calloop::channel::Sender;

use crate::SubMessage;

/// Spawn a one-shot command in a background thread. Sends the full stdout as a
/// single message when the command completes.
pub fn spawn_exec(cmd: &str, msg_name: String, sender: Sender<SubMessage>) {
    let cmd = cmd.to_string();
    let name = msg_name;
    std::thread::spawn(move || {
        match Command::new("sh")
            .arg("-c")
            .arg(&cmd)
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output()
        {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
                let _ = sender.send(SubMessage {
                    msg_type: name,
                    data: Some(stdout),
                });
            }
            Err(e) => {
                log::error!("exec '{}' failed: {e}", cmd);
            }
        }
    });
}
