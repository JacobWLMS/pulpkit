use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use crate::poll::sh;
use crate::state::{DisplayOutput, FullState};

pub fn start_outputs_watcher(state: Arc<Mutex<FullState>>, dirty: Arc<AtomicBool>) {
    std::thread::spawn(move || loop {
        let outputs = read_outputs();
        if let Ok(mut s) = state.lock() {
            s.outputs = outputs;
        }
        dirty.store(true, Ordering::Relaxed);
        std::thread::sleep(std::time::Duration::from_secs(10));
    });
}

fn read_outputs() -> Vec<DisplayOutput> {
    let Some(json_str) = sh("niri msg -j outputs 2>/dev/null") else {
        return Vec::new();
    };

    let Ok(arr) = serde_json::from_str::<Vec<serde_json::Value>>(&json_str) else {
        return Vec::new();
    };

    arr.iter()
        .map(|obj| {
            let mode = obj.get("current_mode").or_else(|| obj.get("mode"));
            let (width, height, refresh) = mode
                .map(|m| {
                    (
                        m.get("width").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                        m.get("height").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                        m.get("refresh_rate")
                            .and_then(|v| v.as_f64())
                            .unwrap_or(0.0) as f32,
                    )
                })
                .unwrap_or((0, 0, 0.0));

            DisplayOutput {
                name: obj.get("name").and_then(|v| v.as_str()).unwrap_or("").into(),
                make: obj.get("make").and_then(|v| v.as_str()).unwrap_or("").into(),
                model: obj.get("model").and_then(|v| v.as_str()).unwrap_or("").into(),
                width,
                height,
                refresh,
                scale: obj.get("scale").and_then(|v| v.as_f64()).unwrap_or(1.0) as f32,
                enabled: obj
                    .get("is_enabled")
                    .or_else(|| obj.get("enabled"))
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true),
            }
        })
        .collect()
}
