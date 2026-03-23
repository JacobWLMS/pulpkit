use std::collections::HashMap;

use crate::icons::resolve_icon;
use crate::state::{AppEntry, WifiNet, WindowInfo, Workspace};

pub fn sh(c: &str) -> Option<String> {
    std::process::Command::new("sh")
        .args(["-c", c])
        .output()
        .ok()
        .and_then(|o| {
            let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
            if s.is_empty() {
                None
            } else {
                Some(s)
            }
        })
}

pub fn poll_vol() -> (u32, bool) {
    let r = sh("wpctl get-volume @DEFAULT_AUDIO_SINK@ 2>/dev/null").unwrap_or_default();
    let muted = r.contains("[MUTED]");
    let vol = r
        .split_whitespace()
        .nth(1)
        .and_then(|v| v.parse::<f64>().ok())
        .map(|v| (v * 100.0) as u32)
        .unwrap_or(0);
    (vol, muted)
}

pub fn poll_audio_device() -> String {
    sh("wpctl inspect @DEFAULT_AUDIO_SINK@ 2>/dev/null | grep 'node.description' | head -1")
        .and_then(|s| s.split('"').nth(1).map(|s| s.to_string()))
        .unwrap_or_default()
}

pub fn poll_mem() -> u32 {
    let c = std::fs::read_to_string("/proc/meminfo").unwrap_or_default();
    let mut total = 0u64;
    let mut avail = 0u64;
    for line in c.lines() {
        if let Some(v) = line.strip_prefix("MemTotal:") {
            total = v
                .trim()
                .split_whitespace()
                .next()
                .and_then(|n| n.parse().ok())
                .unwrap_or(0);
        } else if let Some(v) = line.strip_prefix("MemAvailable:") {
            avail = v
                .trim()
                .split_whitespace()
                .next()
                .and_then(|n| n.parse().ok())
                .unwrap_or(0);
        }
        if total > 0 && avail > 0 {
            break;
        }
    }
    if total == 0 {
        0
    } else {
        ((total - avail) * 100 / total) as u32
    }
}

pub fn poll_wifi() -> String {
    sh("nmcli -t -f ACTIVE,SSID dev wifi 2>/dev/null | grep '^yes' | head -1")
        .and_then(|r| r.strip_prefix("yes:").map(|s| s.to_string()))
        .unwrap_or_default()
}

pub fn poll_bri() -> u32 {
    sh("brightnessctl -m 2>/dev/null")
        .and_then(|r| {
            r.split(',')
                .nth(3)
                .and_then(|s| s.trim_end_matches('%').parse().ok())
        })
        .unwrap_or(50)
}

pub fn poll_cpu() -> u32 {
    fn read_cpu() -> Option<(u64, u64)> {
        let s = std::fs::read_to_string("/proc/stat").ok()?;
        let line = s.lines().next()?;
        let nums: Vec<u64> = line
            .split_whitespace()
            .skip(1)
            .filter_map(|n| n.parse().ok())
            .collect();
        if nums.len() < 4 {
            return None;
        }
        let idle = nums[3];
        let total: u64 = nums.iter().sum();
        Some((idle, total))
    }
    let Some((idle1, total1)) = read_cpu() else {
        return 0;
    };
    std::thread::sleep(std::time::Duration::from_millis(100));
    let Some((idle2, total2)) = read_cpu() else {
        return 0;
    };
    let di = idle2.saturating_sub(idle1);
    let dt = total2.saturating_sub(total1);
    if dt == 0 {
        0
    } else {
        ((dt - di) * 100 / dt) as u32
    }
}

pub fn poll_disk() -> (String, String, u32) {
    sh("df -h / 2>/dev/null | tail -1")
        .map(|line| {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 5 {
                let total = parts[1].to_string();
                let used = parts[2].to_string();
                let pct = parts[4].trim_end_matches('%').parse().unwrap_or(0u32);
                (used, total, pct)
            } else {
                ("0".into(), "0".into(), 0)
            }
        })
        .unwrap_or_else(|| ("0".into(), "0".into(), 0))
}

pub fn poll_net_details() -> (u32, String) {
    let signal = sh("nmcli -t -f ACTIVE,SIGNAL dev wifi 2>/dev/null | grep '^yes'")
        .and_then(|r| r.split(':').nth(1).and_then(|s| s.parse().ok()))
        .unwrap_or(0u32);
    let ip = sh("hostname -I 2>/dev/null")
        .map(|s| s.split_whitespace().next().unwrap_or("").to_string())
        .unwrap_or_default();
    (signal, ip)
}

pub fn poll_ws() -> Vec<Workspace> {
    let r = sh("niri msg -j workspaces 2>/dev/null").unwrap_or_else(|| "[]".into());
    let mut ws = Vec::new();
    for chunk in r.split('{').skip(1) {
        let idx = chunk
            .split("\"idx\":")
            .nth(1)
            .and_then(|s| s.split([',', '}']).next())
            .and_then(|s| s.trim().parse::<u32>().ok());
        let active = chunk.contains("\"is_focused\":true");
        if let Some(idx) = idx {
            ws.push(Workspace { idx, active });
        }
    }
    ws.sort_by_key(|w| w.idx);
    ws
}

pub fn poll_windows(icon_cache: &HashMap<String, String>) -> Vec<WindowInfo> {
    let r = sh("niri msg -j windows 2>/dev/null").unwrap_or_else(|| "[]".into());
    let mut wins = Vec::new();
    for chunk in r.split('{').skip(1) {
        let id = chunk
            .split("\"id\":")
            .nth(1)
            .and_then(|s| s.split([',', '}']).next())
            .and_then(|s| s.trim().parse::<u64>().ok());
        let app_id = chunk
            .split("\"app_id\":\"")
            .nth(1)
            .and_then(|s| s.split('"').next())
            .unwrap_or("")
            .to_string();
        let title = chunk
            .split("\"title\":\"")
            .nth(1)
            .and_then(|s| s.split('"').next())
            .unwrap_or("")
            .to_string();
        let focused = chunk.contains("\"is_focused\":true");
        let icon = icon_cache
            .get(&app_id.to_lowercase())
            .or_else(|| {
                app_id
                    .rsplit('.')
                    .next()
                    .and_then(|last| icon_cache.get(&last.to_lowercase()))
            })
            .cloned()
            .unwrap_or_default();
        if let Some(id) = id {
            wins.push(WindowInfo {
                id,
                title,
                app_id,
                focused,
                icon,
            });
        }
    }
    wins
}

pub fn scan_wifi() -> Vec<WifiNet> {
    let out =
        sh("nmcli -t -f SSID,SIGNAL,SECURITY,ACTIVE dev wifi list 2>/dev/null").unwrap_or_default();
    let mut nets = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for line in out.lines() {
        let parts: Vec<&str> = line.splitn(4, ':').collect();
        if parts.len() >= 4 && !parts[0].is_empty() && seen.insert(parts[0].to_string()) {
            nets.push(WifiNet {
                ssid: parts[0].to_string(),
                signal: parts[1].parse().unwrap_or(0),
                secure: !parts[2].is_empty(),
                active: parts[3] == "yes",
            });
        }
    }
    nets.sort_by(|a, b| b.active.cmp(&a.active).then(b.signal.cmp(&a.signal)));
    nets
}

pub fn scan_apps() -> Vec<AppEntry> {
    let mut apps = Vec::new();
    let dirs = [
        "/usr/share/applications".to_string(),
        format!(
            "{}/.local/share/applications",
            std::env::var("HOME").unwrap_or_default()
        ),
    ];
    for dir in &dirs {
        let Ok(entries) = std::fs::read_dir(dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "desktop") {
                let Ok(content) = std::fs::read_to_string(&path) else {
                    continue;
                };
                let mut name = None;
                let mut exec = None;
                let mut icon_name = None;
                let mut nodisplay = false;
                let mut in_entry = false;
                for line in content.lines() {
                    if line.starts_with('[') {
                        in_entry = line == "[Desktop Entry]";
                        continue;
                    }
                    if !in_entry {
                        continue;
                    }
                    if let Some(v) = line.strip_prefix("Name=") {
                        if name.is_none() {
                            name = Some(v.to_string());
                        }
                    } else if let Some(v) = line.strip_prefix("Exec=") {
                        let clean = v
                            .split_whitespace()
                            .filter(|w| !w.starts_with('%'))
                            .collect::<Vec<_>>()
                            .join(" ");
                        exec = Some(clean);
                    } else if let Some(v) = line.strip_prefix("Icon=") {
                        icon_name = Some(v.to_string());
                    } else if line == "NoDisplay=true" {
                        nodisplay = true;
                    }
                }
                if let (Some(n), Some(e)) = (name, exec) {
                    if !nodisplay {
                        let icon = icon_name
                            .as_deref()
                            .map(resolve_icon)
                            .unwrap_or_default();
                        apps.push(AppEntry {
                            name: n,
                            exec: e,
                            icon,
                        });
                    }
                }
            }
        }
    }
    apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    apps
}

pub fn has_battery() -> bool {
    std::path::Path::new("/sys/class/power_supply/BAT0/capacity").exists()
}
