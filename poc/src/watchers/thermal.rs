use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use crate::state::FullState;

pub fn start_thermal_watcher(state: Arc<Mutex<FullState>>, dirty: Arc<AtomicBool>) {
    std::thread::spawn(move || {
        let zone = find_cpu_thermal_zone()
            .unwrap_or_else(|| "/sys/class/thermal/thermal_zone0".to_string());

        loop {
            let temp = read_temp(&zone);
            if let Ok(mut s) = state.lock() {
                s.cpu_temp = temp;
            }
            dirty.store(true, Ordering::Relaxed);
            std::thread::sleep(std::time::Duration::from_secs(3));
        }
    });
}

fn find_cpu_thermal_zone() -> Option<String> {
    for entry in std::fs::read_dir("/sys/class/thermal/").ok()? {
        let entry = entry.ok()?;
        let path = entry.path();
        if !path.file_name()?.to_str()?.starts_with("thermal_zone") {
            continue;
        }
        let type_path = path.join("type");
        if let Ok(zone_type) = std::fs::read_to_string(&type_path) {
            let t = zone_type.trim();
            if t.contains("x86_pkg_temp") || t.contains("coretemp") || t.contains("acpitz") {
                return Some(path.to_string_lossy().to_string());
            }
        }
    }
    // Fallback to zone0
    let z0 = "/sys/class/thermal/thermal_zone0";
    if std::path::Path::new(z0).exists() {
        Some(z0.to_string())
    } else {
        None
    }
}

fn read_temp(zone_path: &str) -> u32 {
    std::fs::read_to_string(format!("{zone_path}/temp"))
        .ok()
        .and_then(|s| s.trim().parse::<u32>().ok())
        .map(|millideg| millideg / 1000)
        .unwrap_or(0)
}
