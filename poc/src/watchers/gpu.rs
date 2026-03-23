use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use crate::state::FullState;

fn read_sysfs(path: &str) -> Option<String> {
    std::fs::read_to_string(path)
        .ok()
        .map(|s| s.trim().to_string())
}

fn read_sysfs_u64(path: &str) -> Option<u64> {
    read_sysfs(path).and_then(|s| s.parse().ok())
}

fn read_sysfs_u32(path: &str) -> Option<u32> {
    read_sysfs(path).and_then(|s| s.parse().ok())
}

/// Find the card path under /sys/class/drm/ for an AMD or NVIDIA GPU.
/// Returns something like "/sys/class/drm/card1/device".
fn detect_gpu_card() -> Option<String> {
    let Ok(entries) = std::fs::read_dir("/sys/class/drm") else {
        return None;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        // Only look at cardN entries (not cardN-DP-1 etc.)
        if !name.starts_with("card") || name.contains('-') {
            continue;
        }
        let device_path = format!("/sys/class/drm/{name}/device");
        let vendor_path = format!("{device_path}/vendor");
        if let Some(vendor) = read_sysfs(&vendor_path) {
            // AMD = 0x1002, NVIDIA = 0x10de
            if vendor == "0x1002" || vendor == "0x10de" {
                return Some(device_path);
            }
        }
    }
    None
}

/// Find the hwmon path for the GPU by scanning /sys/class/hwmon/hwmon* and
/// checking if the `name` file contains "amdgpu" or "nvidia".
fn detect_gpu_hwmon() -> Option<String> {
    let Ok(entries) = std::fs::read_dir("/sys/class/hwmon") else {
        return None;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name_path = format!("{}/name", path.display());
        if let Some(name) = read_sysfs(&name_path) {
            if name.contains("amdgpu") || name.contains("nvidia") {
                return Some(path.to_string_lossy().to_string());
            }
        }
    }
    None
}

pub fn start_gpu_watcher(state: Arc<Mutex<FullState>>, dirty: Arc<AtomicBool>) {
    std::thread::spawn(move || {
        let Some(device_path) = detect_gpu_card() else {
            eprintln!("[pulpkit] gpu: no AMD/NVIDIA GPU found in /sys/class/drm");
            return;
        };
        eprintln!("[pulpkit] gpu: monitoring {device_path}");

        let hwmon_path = detect_gpu_hwmon();
        if hwmon_path.is_none() {
            eprintln!("[pulpkit] gpu: no hwmon found for GPU temperature");
        }

        let usage_path = format!("{device_path}/gpu_busy_percent");
        let vram_used_path = format!("{device_path}/mem_info_vram_used");
        let vram_total_path = format!("{device_path}/mem_info_vram_total");
        let temp_path = hwmon_path.map(|h| format!("{h}/temp1_input"));

        loop {
            let gpu_usage = read_sysfs_u32(&usage_path).unwrap_or(0);

            let gpu_temp = temp_path
                .as_deref()
                .and_then(read_sysfs_u64)
                .map(|millideg| (millideg / 1000) as u32)
                .unwrap_or(0);

            let vram_used_mb = read_sysfs_u64(&vram_used_path)
                .map(|b| (b / (1024 * 1024)) as u32)
                .unwrap_or(0);

            let vram_total_mb = read_sysfs_u64(&vram_total_path)
                .map(|b| (b / (1024 * 1024)) as u32)
                .unwrap_or(0);

            {
                let mut s = state.lock().unwrap();
                s.gpu_usage = gpu_usage;
                s.gpu_temp = gpu_temp;
                s.vram_used_mb = vram_used_mb;
                s.vram_total_mb = vram_total_mb;
            }
            dirty.store(true, Ordering::Relaxed);

            std::thread::sleep(std::time::Duration::from_secs(2));
        }
    });
}
