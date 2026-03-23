use std::io::{BufRead, Write as IoWrite};
use std::os::unix::net::UnixListener;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

pub type IpcMsg = (String, std::sync::mpsc::Sender<String>);

pub fn start_ipc_server(
    ipc_tx: std::sync::mpsc::Sender<IpcMsg>,
    _dirty: Arc<AtomicBool>,
) {
    let sock_path = format!(
        "{}/pulpkit.sock",
        std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".into())
    );
    let _ = std::fs::remove_file(&sock_path);

    std::thread::spawn(move || {
        let listener = match UnixListener::bind(&sock_path) {
            Ok(l) => l,
            Err(e) => {
                eprintln!("[pulpkit] IPC bind failed: {e}");
                return;
            }
        };
        eprintln!("[pulpkit] IPC socket: {sock_path}");

        for stream in listener.incoming().flatten() {
            let tx = ipc_tx.clone();
            std::thread::spawn(move || {
                let reader = std::io::BufReader::new(match stream.try_clone() {
                    Ok(s) => s,
                    Err(_) => return,
                });
                let mut writer = stream;
                for line in reader.lines().flatten() {
                    let trimmed = line.trim().to_string();
                    if trimmed.is_empty() {
                        continue;
                    }
                    if trimmed.starts_with('{') {
                        let (resp_tx, resp_rx) = std::sync::mpsc::channel();
                        if tx.send((trimmed, resp_tx)).is_err() {
                            break;
                        }
                        let response = resp_rx
                            .recv_timeout(std::time::Duration::from_secs(10))
                            .unwrap_or_else(|_| {
                                r#"{"ok":false,"error":"timeout"}"#.into()
                            });
                        if writeln!(writer, "{response}").is_err() {
                            break;
                        }
                    }
                }
            });
        }
    });
}
