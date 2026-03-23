use std::io::BufRead;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use crate::poll::sh;
use crate::state::{AudioDevice, FullState};

fn read_devices(state: &Arc<Mutex<FullState>>, dirty: &Arc<AtomicBool>) {
    let default_sink = sh("pactl get-default-sink 2>/dev/null").unwrap_or_default();
    let default_source = sh("pactl get-default-source 2>/dev/null").unwrap_or_default();

    let sinks = parse_pactl_devices("sinks", &default_sink);
    let sources = parse_pactl_devices("sources", &default_source);

    if let Ok(mut s) = state.lock() {
        s.audio_sinks = sinks;
        s.audio_sources = sources;
    }
    dirty.store(true, Ordering::Relaxed);
}

fn parse_pactl_devices(kind: &str, default_name: &str) -> Vec<AudioDevice> {
    let output = sh(&format!("pactl -f json list {kind} 2>/dev/null")).unwrap_or_default();
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap_or_default();

    let mut devices = vec![];
    if let Some(arr) = parsed.as_array() {
        for item in arr {
            let name = item["name"].as_str().unwrap_or("").to_string();
            let desc = item["description"].as_str().unwrap_or("").to_string();
            let muted = item["mute"].as_bool().unwrap_or(false);

            // Volume: item["volume"] is an object like {"front-left": {"value": 65536, "value_percent": "100%"}}
            // Get the first channel's value_percent
            let vol = item["volume"]
                .as_object()
                .and_then(|obj| obj.values().next())
                .and_then(|ch| ch["value_percent"].as_str())
                .and_then(|s| s.trim_end_matches('%').parse::<u32>().ok())
                .unwrap_or(0);

            let active = name == default_name;
            devices.push(AudioDevice {
                name,
                description: desc,
                active,
                volume: vol,
                muted,
            });
        }
    }
    devices
}

pub fn start_audio_devices_watcher(state: Arc<Mutex<FullState>>, dirty: Arc<AtomicBool>) {
    std::thread::spawn(move || {
        read_devices(&state, &dirty);

        let Ok(mut child) = Command::new("pactl")
            .arg("subscribe")
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
        else {
            eprintln!("[pulpkit] pactl subscribe (devices) failed");
            return;
        };
        let stdout = child.stdout.take().unwrap();
        let reader = std::io::BufReader::new(stdout);
        for line in reader.lines() {
            let Ok(line) = line else { break };
            if line.contains("sink") || line.contains("source") || line.contains("server") {
                read_devices(&state, &dirty);
            }
        }
    });
}
