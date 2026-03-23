use std::io::BufRead;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use crate::state::FullState;

fn count_trash() -> u32 {
    let trash_dir = format!("{}/.local/share/Trash/files", std::env::var("HOME").unwrap_or_default());
    std::fs::read_dir(&trash_dir)
        .map(|entries| entries.count() as u32)
        .unwrap_or(0)
}

pub fn start_trash_watcher(state: Arc<Mutex<FullState>>, dirty: Arc<AtomicBool>) {
    std::thread::spawn(move || {
        let home = std::env::var("HOME").unwrap_or_default();
        let trash_dir = format!("{home}/.local/share/Trash/files");

        // Ensure trash dir exists
        let _ = std::fs::create_dir_all(&trash_dir);

        // Initial count
        if let Ok(mut s) = state.lock() {
            s.trash_count = count_trash();
        }
        dirty.store(true, Ordering::Relaxed);

        let Ok(mut child) = Command::new("inotifywait")
            .args(["-m", "-e", "create,delete,moved_to,moved_from", &trash_dir])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
        else {
            eprintln!("[pulpkit] inotifywait for trash failed to start");
            return;
        };

        let stdout = child.stdout.take().unwrap();
        let reader = std::io::BufReader::new(stdout);
        for _line in reader.lines().flatten() {
            if let Ok(mut s) = state.lock() {
                s.trash_count = count_trash();
            }
            dirty.store(true, Ordering::Relaxed);
        }
    });
}
