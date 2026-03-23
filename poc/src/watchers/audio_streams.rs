use std::io::BufRead;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use crate::state::{AudioStream, FullState};
use crate::poll::sh;

fn read_streams() -> Vec<AudioStream> {
    let mut streams = vec![];
    for (kind, is_input) in [("sink-inputs", false), ("source-outputs", true)] {
        let output = sh(&format!("pactl -f json list {kind} 2>/dev/null")).unwrap_or_default();
        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap_or_default();
        if let Some(arr) = parsed.as_array() {
            for item in arr {
                let props = &item["properties"];
                let app_name = props["application.name"].as_str().unwrap_or("").to_string();
                let name = props["media.name"].as_str().unwrap_or(&app_name).to_string();
                let muted = item["mute"].as_bool().unwrap_or(false);
                let vol = item["volume"].as_object()
                    .and_then(|o| o.values().next())
                    .and_then(|ch| ch["value_percent"].as_str())
                    .and_then(|s| s.trim_end_matches('%').parse::<u32>().ok())
                    .unwrap_or(0);
                streams.push(AudioStream { name, app_name, volume: vol, muted, is_input });
            }
        }
    }
    streams
}

pub fn start_audio_streams_watcher(state: Arc<Mutex<FullState>>, dirty: Arc<AtomicBool>) {
    std::thread::spawn(move || {
        if let Ok(mut s) = state.lock() { s.audio_streams = read_streams(); }
        dirty.store(true, Ordering::Relaxed);

        let Ok(mut child) = Command::new("pactl").arg("subscribe").stdout(Stdio::piped()).stderr(Stdio::null()).spawn()
        else { return; };
        let stdout = child.stdout.take().unwrap();
        for line in std::io::BufReader::new(stdout).lines().flatten() {
            if line.contains("sink-input") || line.contains("source-output") {
                if let Ok(mut s) = state.lock() { s.audio_streams = read_streams(); }
                dirty.store(true, Ordering::Relaxed);
            }
        }
    });
}
