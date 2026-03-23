use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use crate::poll::sh;
use crate::state::{ContainerInfo, FullState};

pub fn start_containers_watcher(state: Arc<Mutex<FullState>>, dirty: Arc<AtomicBool>) {
    std::thread::spawn(move || loop {
        let containers = read_containers();
        if let Ok(mut s) = state.lock() {
            s.containers = containers;
        }
        dirty.store(true, Ordering::Relaxed);
        std::thread::sleep(std::time::Duration::from_secs(10));
    });
}

fn read_containers() -> Vec<ContainerInfo> {
    // Try podman first, then docker
    let output = sh("podman ps --format json 2>/dev/null")
        .or_else(|| sh("docker ps --format json 2>/dev/null"));

    let Some(raw) = output else {
        return vec![];
    };

    // podman outputs a JSON array; docker outputs one JSON object per line.
    // Try parsing as array first, then fall back to newline-delimited JSON.
    if let Ok(arr) = serde_json::from_str::<Vec<serde_json::Value>>(&raw) {
        return arr.iter().filter_map(parse_container).collect();
    }

    // Newline-delimited JSON (docker default)
    raw.lines()
        .filter_map(|line| {
            let val: serde_json::Value = serde_json::from_str(line).ok()?;
            parse_container(&val)
        })
        .collect()
}

fn parse_container(val: &serde_json::Value) -> Option<ContainerInfo> {
    // Podman uses "Names" (string), docker uses "Names" (string).
    // Podman also sometimes uses "Name" (singular).
    let name = val
        .get("Names")
        .or_else(|| val.get("Name"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let image = val
        .get("Image")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let status = val
        .get("Status")
        .or_else(|| val.get("State"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let id = val
        .get("ID")
        .or_else(|| val.get("Id"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .chars()
        .take(12)
        .collect::<String>();

    if name.is_empty() && id.is_empty() {
        return None;
    }

    Some(ContainerInfo {
        name,
        image,
        status,
        id,
    })
}
