//! Stream subscription — spawns a subprocess and reads stdout line-by-line.

use std::io::BufRead;
use std::process::{Command, Stdio};

use calloop::channel::{self, Sender};

use crate::SubMessage;

/// Spawn a stream subprocess. Returns a calloop channel receiver and the child PID.
///
/// A background thread reads stdout line-by-line and sends each line through
/// the channel, which wakes the calloop event loop.
pub fn spawn_stream(
    cmd: &str,
    msg_name: String,
    sender: Sender<SubMessage>,
) -> Option<(channel::Channel<SubMessage>, u32)> {
    let child = Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;

    let pid = child.id();
    let (line_sender, line_receiver) = channel::channel::<SubMessage>();

    let name = msg_name;
    std::thread::spawn(move || {
        let mut child = child;
        if let Some(stdout) = child.stdout.take() {
            let reader = std::io::BufReader::new(stdout);
            for line in reader.lines() {
                match line {
                    Ok(line) => {
                        // Send to the calloop channel (wakes the event loop)
                        if sender.send(SubMessage {
                            msg_type: name.clone(),
                            data: Some(line),
                        }).is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        }
        let _ = child.wait();
        log::info!("Stream '{}' ended", name);
    });

    Some((line_receiver, pid))
}
