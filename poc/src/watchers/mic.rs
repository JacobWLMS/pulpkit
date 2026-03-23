use std::io::BufRead;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use crate::state::FullState;
use crate::poll::sh;

fn update_mic(state: &Arc<Mutex<FullState>>, dirty: &Arc<AtomicBool>) {
    let r = sh("wpctl get-volume @DEFAULT_AUDIO_SOURCE@ 2>/dev/null").unwrap_or_default();
    let muted = r.contains("[MUTED]");
    let vol = r.split_whitespace().nth(1)
        .and_then(|v| v.parse::<f64>().ok())
        .map(|v| (v * 100.0) as u32).unwrap_or(0);
    if let Ok(mut s) = state.lock() {
        s.mic_muted = muted;
        s.mic_volume = vol;
    }
    dirty.store(true, Ordering::Relaxed);
}

pub fn start_mic_watcher(state: Arc<Mutex<FullState>>, dirty: Arc<AtomicBool>) {
    std::thread::spawn(move || {
        update_mic(&state, &dirty);
        let Ok(mut child) = Command::new("pactl").arg("subscribe").stdout(Stdio::piped()).stderr(Stdio::null()).spawn()
        else { return; };
        let stdout = child.stdout.take().unwrap();
        for line in std::io::BufReader::new(stdout).lines().flatten() {
            if line.contains("source") || line.contains("server") {
                update_mic(&state, &dirty);
            }
        }
    });
}
