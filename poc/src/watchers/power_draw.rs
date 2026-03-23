use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use crate::state::FullState;

pub fn start_power_draw_watcher(state: Arc<Mutex<FullState>>, dirty: Arc<AtomicBool>) {
    std::thread::spawn(move || loop {
        let watts = read_power_draw();
        if let Ok(mut s) = state.lock() {
            s.power_draw_watts = watts;
        }
        dirty.store(true, Ordering::Relaxed);
        std::thread::sleep(std::time::Duration::from_secs(3));
    });
}

fn read_power_draw() -> f32 {
    // Try power_now first (reports in microwatts)
    if let Some(uw) = read_sysfs_u64("/sys/class/power_supply/BAT0/power_now") {
        return uw as f32 / 1_000_000.0;
    }

    // Fallback: compute from voltage_now (uV) * current_now (uA)
    let voltage_uv = read_sysfs_u64("/sys/class/power_supply/BAT0/voltage_now");
    let current_ua = read_sysfs_u64("/sys/class/power_supply/BAT0/current_now");

    match (voltage_uv, current_ua) {
        (Some(v), Some(i)) => {
            // P = V * I; both in micro-units so result is in pico-watts
            // Divide by 1e12 to get watts
            (v as f64 * i as f64 / 1_000_000_000_000.0) as f32
        }
        _ => 0.0,
    }
}

fn read_sysfs_u64(path: &str) -> Option<u64> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| s.trim().parse().ok())
}
