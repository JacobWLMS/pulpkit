use std::io::BufRead;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use crate::poll::{poll_audio_device, poll_vol};
use crate::state::FullState;

fn update_audio_state(state: &Arc<Mutex<FullState>>, dirty: &Arc<AtomicBool>) {
    let (vol, muted) = poll_vol();
    let audio_device = poll_audio_device();
    if let Ok(mut s) = state.lock() {
        s.vol = vol;
        s.muted = muted;
        s.audio_device = audio_device;
    }
    dirty.store(true, Ordering::Relaxed);
}

pub fn start_audio_watcher(state: Arc<Mutex<FullState>>, dirty: Arc<AtomicBool>) {
    std::thread::spawn(move || {
        // Initial read so we have audio state before any events arrive
        update_audio_state(&state, &dirty);

        let Ok(mut child) = Command::new("pactl")
            .arg("subscribe")
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
        else {
            eprintln!("[pulpkit] pactl subscribe failed to start");
            return;
        };

        let stdout = child.stdout.take().unwrap();
        let reader = std::io::BufReader::new(stdout);
        for line in reader.lines() {
            let Ok(line) = line else { break };
            if line.contains("sink") || line.contains("server") || line.contains("source") {
                update_audio_state(&state, &dirty);
            }
        }
    });
}
