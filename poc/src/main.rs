use std::cell::RefCell;
use std::io::{BufRead, Write as IoWrite};
use std::os::unix::net::UnixListener;
use std::process::{Command, Stdio};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use gtk4::glib;
use gtk4::prelude::*;
use gtk4::Application;
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
use serde::Serialize;
use webkit6::prelude::*;
use webkit6::{UserContentManager, WebView};

const DEFAULT_BAR_HEIGHT: i32 = 40;
const DEFAULT_POPUP_WIDTH: i32 = 380;
const DEFAULT_POPUP_HEIGHT: i32 = 500;

#[derive(serde::Deserialize)]
struct ThemeConfig {
    #[serde(default = "default_bar_height")]
    bar_height: i32,
    #[serde(default = "default_popup_width")]
    popup_width: i32,
    #[serde(default = "default_popup_height")]
    popup_height: i32,
    #[serde(default)]
    bar_position: String, // "top" or "bottom"
}

fn default_bar_height() -> i32 { DEFAULT_BAR_HEIGHT }
fn default_popup_width() -> i32 { DEFAULT_POPUP_WIDTH }
fn default_popup_height() -> i32 { DEFAULT_POPUP_HEIGHT }

impl Default for ThemeConfig {
    fn default() -> Self {
        Self { bar_height: DEFAULT_BAR_HEIGHT, popup_width: DEFAULT_POPUP_WIDTH, popup_height: DEFAULT_POPUP_HEIGHT, bar_position: "top".into() }
    }
}

// ── System polling ─────────────────────────────────

fn sh(c: &str) -> Option<String> {
    Command::new("sh").args(["-c", c]).output().ok().and_then(|o| {
        let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
        if s.is_empty() { None } else { Some(s) }
    })
}

/// Fire-and-forget command with suppressed output
fn spawn_quiet(cmd: &str, args: &[&str]) {
    let _ = Command::new(cmd).args(args)
        .stdout(Stdio::null()).stderr(Stdio::null()).spawn();
}

fn read_file(p: &str) -> Option<String> {
    std::fs::read_to_string(p).ok().map(|s| s.trim().to_string())
}

fn poll_vol() -> (u32, bool) {
    let r = sh("wpctl get-volume @DEFAULT_AUDIO_SINK@ 2>/dev/null").unwrap_or_default();
    let muted = r.contains("[MUTED]");
    let vol = r.split_whitespace().nth(1)
        .and_then(|v| v.parse::<f64>().ok())
        .map(|v| (v * 100.0) as u32).unwrap_or(0);
    (vol, muted)
}

fn poll_bat() -> (u32, String) {
    let cap = read_file("/sys/class/power_supply/BAT0/capacity")
        .and_then(|s| s.parse().ok()).unwrap_or(100u32);
    let st = read_file("/sys/class/power_supply/BAT0/status").unwrap_or_else(|| "Unknown".into());
    (cap, st)
}

fn poll_mem() -> u32 {
    let c = std::fs::read_to_string("/proc/meminfo").unwrap_or_default();
    let mut total = 0u64; let mut avail = 0u64;
    for line in c.lines() {
        if let Some(v) = line.strip_prefix("MemTotal:") {
            total = v.trim().split_whitespace().next().and_then(|n| n.parse().ok()).unwrap_or(0);
        } else if let Some(v) = line.strip_prefix("MemAvailable:") {
            avail = v.trim().split_whitespace().next().and_then(|n| n.parse().ok()).unwrap_or(0);
        }
        if total > 0 && avail > 0 { break; }
    }
    if total == 0 { 0 } else { ((total - avail) * 100 / total) as u32 }
}

fn poll_wifi() -> String {
    sh("nmcli -t -f ACTIVE,SSID dev wifi 2>/dev/null | grep '^yes' | head -1")
        .and_then(|r| r.strip_prefix("yes:").map(|s| s.to_string()))
        .unwrap_or_default()
}

fn poll_notifications() -> (u32, bool) {
    let list = sh("makoctl list 2>/dev/null").unwrap_or_default();
    let count = list.lines().filter(|l| l.starts_with("Notification ")).count() as u32;
    let mode = sh("makoctl mode 2>/dev/null").unwrap_or_default();
    let dnd = mode.trim() == "do-not-disturb";
    (count, dnd)
}

fn poll_bri() -> u32 {
    sh("brightnessctl -m 2>/dev/null")
        .and_then(|r| r.split(',').nth(3).and_then(|s| s.trim_end_matches('%').parse().ok()))
        .unwrap_or(50)
}

fn poll_cpu() -> u32 {
    // Read /proc/stat for aggregate CPU — compare two samples 100ms apart
    fn read_cpu() -> Option<(u64, u64)> {
        let s = std::fs::read_to_string("/proc/stat").ok()?;
        let line = s.lines().next()?;
        let nums: Vec<u64> = line.split_whitespace().skip(1).filter_map(|n| n.parse().ok()).collect();
        if nums.len() < 4 { return None; }
        let idle = nums[3];
        let total: u64 = nums.iter().sum();
        Some((idle, total))
    }
    let Some((idle1, total1)) = read_cpu() else { return 0 };
    std::thread::sleep(std::time::Duration::from_millis(100));
    let Some((idle2, total2)) = read_cpu() else { return 0 };
    let di = idle2.saturating_sub(idle1);
    let dt = total2.saturating_sub(total1);
    if dt == 0 { 0 } else { ((dt - di) * 100 / dt) as u32 }
}

fn poll_disk() -> (String, String, u32) {
    // Returns (used, total, percent) for root filesystem
    sh("df -h / 2>/dev/null | tail -1")
        .map(|line| {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 5 {
                let total = parts[1].to_string();
                let used = parts[2].to_string();
                let pct = parts[4].trim_end_matches('%').parse().unwrap_or(0u32);
                (used, total, pct)
            } else { ("0".into(), "0".into(), 0) }
        })
        .unwrap_or_else(|| ("0".into(), "0".into(), 0))
}

fn poll_power_profile() -> String {
    sh("powerprofilesctl get 2>/dev/null")
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "balanced".into())
}

fn poll_audio_device() -> String {
    sh("wpctl inspect @DEFAULT_AUDIO_SINK@ 2>/dev/null | grep 'node.description' | head -1")
        .and_then(|s| s.split('"').nth(1).map(|s| s.to_string()))
        .unwrap_or_default()
}

fn poll_net_details() -> (u32, String) {
    let signal = sh("nmcli -t -f ACTIVE,SIGNAL dev wifi 2>/dev/null | grep '^yes'")
        .and_then(|r| r.split(':').nth(1).and_then(|s| s.parse().ok()))
        .unwrap_or(0u32);
    let ip = sh("hostname -I 2>/dev/null")
        .map(|s| s.split_whitespace().next().unwrap_or("").to_string())
        .unwrap_or_default();
    (signal, ip)
}

#[derive(Serialize, Clone)]
struct Workspace { idx: u32, active: bool }

fn poll_ws() -> Vec<Workspace> {
    let r = sh("niri msg -j workspaces 2>/dev/null").unwrap_or_else(|| "[]".into());
    let mut ws = Vec::new();
    for chunk in r.split('{').skip(1) {
        let idx = chunk.split("\"idx\":").nth(1)
            .and_then(|s| s.split([',', '}']).next())
            .and_then(|s| s.trim().parse::<u32>().ok());
        let active = chunk.contains("\"is_focused\":true");
        if let Some(idx) = idx { ws.push(Workspace { idx, active }); }
    }
    ws.sort_by_key(|w| w.idx);
    ws
}

#[derive(Serialize, Clone)]
struct WindowInfo { id: u64, title: String, app_id: String, focused: bool, icon: String }

fn resolve_icon(name: &str) -> String {
    if name.is_empty() { return String::new(); }
    // Direct path
    if name.starts_with('/') && std::path::Path::new(name).exists() {
        return format!("file://{name}");
    }
    let search = [
        format!("/usr/share/icons/Papirus-Dark/48x48/apps/{name}.svg"),
        format!("/usr/share/icons/Papirus-Dark/64x64/apps/{name}.svg"),
        format!("/usr/share/icons/Papirus/48x48/apps/{name}.svg"),
        format!("/usr/share/icons/hicolor/scalable/apps/{name}.svg"),
        format!("/usr/share/icons/hicolor/scalable/apps/{name}.svgz"),
        format!("/usr/share/icons/hicolor/48x48/apps/{name}.png"),
        format!("/usr/share/icons/hicolor/256x256/apps/{name}.png"),
        format!("/usr/share/pixmaps/{name}.png"),
        format!("/usr/share/pixmaps/{name}.svg"),
    ];
    // Try exact name first
    for p in &search {
        if std::path::Path::new(p).exists() { return format!("file://{p}"); }
    }
    // Try lowercase
    let lower = name.to_lowercase();
    if lower != name {
        for p in &search {
            let pl = p.replace(name, &lower);
            if std::path::Path::new(&pl).exists() { return format!("file://{pl}"); }
        }
    }
    // Try last segment of dotted app_id (e.g. com.mitchellh.ghostty -> ghostty)
    if let Some(last) = name.rsplit('.').next() {
        if last != name {
            return resolve_icon(last);
        }
    }
    String::new()
}

// Cache: app_id -> icon path
fn build_icon_cache() -> std::collections::HashMap<String, String> {
    let mut cache = std::collections::HashMap::new();
    let dirs = ["/usr/share/applications", &format!("{}/.local/share/applications", std::env::var("HOME").unwrap_or_default())];
    for dir in &dirs {
        let Ok(entries) = std::fs::read_dir(dir) else { continue };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "desktop") {
                let Ok(content) = std::fs::read_to_string(&path) else { continue };
                let mut startup_wm = None;
                let mut icon_name = None;
                let mut in_entry = false;
                for line in content.lines() {
                    if line.starts_with('[') { in_entry = line == "[Desktop Entry]"; continue; }
                    if !in_entry { continue; }
                    if let Some(v) = line.strip_prefix("Icon=") { icon_name = Some(v.to_string()); }
                    else if let Some(v) = line.strip_prefix("StartupWMClass=") { startup_wm = Some(v.to_lowercase()); }
                }
                // Use filename without .desktop as fallback key
                let fname = path.file_stem().map(|s| s.to_string_lossy().to_lowercase());
                if let Some(icon) = &icon_name {
                    let resolved = resolve_icon(icon);
                    if !resolved.is_empty() {
                        if let Some(ref wm) = startup_wm { cache.insert(wm.clone(), resolved.clone()); }
                        if let Some(ref f) = fname { cache.insert(f.clone(), resolved.clone()); }
                        // Also index by icon name itself
                        cache.insert(icon.to_lowercase(), resolved);
                    }
                }
            }
        }
    }
    cache
}

fn poll_windows(icon_cache: &std::collections::HashMap<String, String>) -> Vec<WindowInfo> {
    let r = sh("niri msg -j windows 2>/dev/null").unwrap_or_else(|| "[]".into());
    let mut wins = Vec::new();
    for chunk in r.split('{').skip(1) {
        let id = chunk.split("\"id\":").nth(1)
            .and_then(|s| s.split([',', '}']).next())
            .and_then(|s| s.trim().parse::<u64>().ok());
        let app_id = chunk.split("\"app_id\":\"").nth(1)
            .and_then(|s| s.split('"').next())
            .unwrap_or("").to_string();
        let title = chunk.split("\"title\":\"").nth(1)
            .and_then(|s| s.split('"').next())
            .unwrap_or("").to_string();
        let focused = chunk.contains("\"is_focused\":true");
        let icon = icon_cache.get(&app_id.to_lowercase())
            .or_else(|| {
                // Try last segment of dotted app_id
                app_id.rsplit('.').next().and_then(|last| icon_cache.get(&last.to_lowercase()))
            })
            .cloned()
            .unwrap_or_default();
        if let Some(id) = id {
            wins.push(WindowInfo { id, title, app_id, focused, icon });
        }
    }
    wins
}

#[derive(Serialize, Clone)]
struct WifiNet { ssid: String, signal: u32, secure: bool, active: bool }

fn scan_wifi() -> Vec<WifiNet> {
    let out = sh("nmcli -t -f SSID,SIGNAL,SECURITY,ACTIVE dev wifi list 2>/dev/null").unwrap_or_default();
    let mut nets = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for line in out.lines() {
        let parts: Vec<&str> = line.splitn(4, ':').collect();
        if parts.len() >= 4 && !parts[0].is_empty() && seen.insert(parts[0].to_string()) {
            nets.push(WifiNet {
                ssid: parts[0].to_string(), signal: parts[1].parse().unwrap_or(0),
                secure: !parts[2].is_empty(), active: parts[3] == "yes",
            });
        }
    }
    nets.sort_by(|a, b| b.active.cmp(&a.active).then(b.signal.cmp(&a.signal)));
    nets
}

#[derive(Serialize, Clone)]
struct AppEntry { name: String, exec: String, icon: String }

fn scan_apps() -> Vec<AppEntry> {
    let mut apps = Vec::new();
    let dirs = ["/usr/share/applications", &format!("{}/.local/share/applications", std::env::var("HOME").unwrap_or_default())];
    for dir in &dirs {
        let Ok(entries) = std::fs::read_dir(dir) else { continue };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "desktop") {
                let Ok(content) = std::fs::read_to_string(&path) else { continue };
                let mut name = None; let mut exec = None; let mut icon_name = None; let mut nodisplay = false;
                let mut in_entry = false;
                for line in content.lines() {
                    if line.starts_with('[') { in_entry = line == "[Desktop Entry]"; continue; }
                    if !in_entry { continue; }
                    if let Some(v) = line.strip_prefix("Name=") { if name.is_none() { name = Some(v.to_string()); } }
                    else if let Some(v) = line.strip_prefix("Exec=") {
                        let clean = v.split_whitespace()
                            .filter(|w| !w.starts_with('%'))
                            .collect::<Vec<_>>().join(" ");
                        exec = Some(clean);
                    }
                    else if let Some(v) = line.strip_prefix("Icon=") { icon_name = Some(v.to_string()); }
                    else if line == "NoDisplay=true" { nodisplay = true; }
                }
                if let (Some(n), Some(e)) = (name, exec) {
                    if !nodisplay {
                        let icon = icon_name.as_deref().map(resolve_icon).unwrap_or_default();
                        apps.push(AppEntry { name: n, exec: e, icon });
                    }
                }
            }
        }
    }
    apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    apps
}

fn has_battery() -> bool {
    std::path::Path::new("/sys/class/power_supply/BAT0/capacity").exists()
}

// ── State ──────────────────────────────────────────

#[derive(Serialize, Clone, Default)]
struct FullState {
    // Audio
    vol: u32, muted: bool, audio_device: String,
    // Display
    bright: u32,
    // Battery
    bat: u32, bat_status: String, has_bat: bool,
    // System
    cpu: u32, mem: u32,
    disk_used: String, disk_total: String, disk_pct: u32,
    power_profile: String,
    // Network
    wifi: String, net_signal: u32, net_ip: String,
    // Notifications
    notif_count: u32, dnd: bool,
    // Workspaces & windows
    ws: Vec<Workspace>,
    windows: Vec<WindowInfo>,
    active_title: String, active_app_id: String,
    // Dynamic lists (populated on demand)
    wifi_nets: Vec<WifiNet>,
    apps: Vec<AppEntry>,
    tray_items: Vec<TrayItem>,
    // UI state
    popup: String,
    theme: String,
    // Custom key-value store (for shell-specific state)
    custom: std::collections::HashMap<String, serde_json::Value>,
    // Static system info
    user: String, host: String, kernel: String, uptime: String,
}

// ── Shared app state ───────────────────────────────

#[derive(Serialize, Clone)]
struct TrayItem { id: String, address: String, title: String, icon: String }

struct AppState {
    popup: String,
    theme: String,
    wifi_nets: Vec<WifiNet>,
    apps: Vec<AppEntry>,
    tray_items: Vec<TrayItem>,
    custom: std::collections::HashMap<String, serde_json::Value>,
    dirty: Arc<AtomicBool>,
    tray_activate_tx: Option<tokio::sync::mpsc::Sender<(String, String)>>,
}

// ── Command handler ────────────────────────────────

fn handle_command(cmd_str: &str, state: &Rc<RefCell<AppState>>) {
    let parsed: serde_json::Value = match serde_json::from_str(cmd_str) {
        Ok(v) => v, Err(e) => { eprintln!("[pulpkit] bad cmd: {e}"); return; }
    };
    let c = parsed["cmd"].as_str().unwrap_or("");
    let data = &parsed["data"];

    let mut s = state.borrow_mut();
    match c {
        "ws_go" => { if let Some(idx) = data.as_u64() { spawn_quiet("niri", &["msg", "action", "focus-workspace", &idx.to_string()]); } }
        "focus_window" => { if let Some(id) = data.as_u64() { spawn_quiet("niri", &["msg", "action", "focus-window", "--id", &id.to_string()]); } }
        "vol_set" => { if let Some(v) = data.as_f64() { spawn_quiet("wpctl", &["set-volume", "@DEFAULT_AUDIO_SINK@", &format!("{:.2}", v / 100.0)]); } }
        "vol_mute" => { spawn_quiet("wpctl", &["set-mute", "@DEFAULT_AUDIO_SINK@", "toggle"]); }
        "bri_set" => { if let Some(v) = data.as_u64() { spawn_quiet("brightnessctl", &["set", &format!("{v}%")]); } }
        "wifi_con" => { if let Some(ssid) = data.as_str() { spawn_quiet("nmcli", &["dev", "wifi", "connect", ssid]); } }
        "wifi_dis" => { spawn_quiet("nmcli", &["dev", "disconnect", "wlan0"]); }
        "popup" => {
            let target = data.as_str().unwrap_or("");
            if s.popup == target {
                s.popup.clear();
            } else {
                s.popup = target.to_string();
                if target == "wifi" { s.wifi_nets = scan_wifi(); }
                if target == "launcher" { s.apps = scan_apps(); }
            }
            s.dirty.store(true, Ordering::Relaxed);
        }
        "dismiss" => { s.popup.clear(); s.dirty.store(true, Ordering::Relaxed); }
        "launch" => {
            if let Some(exec) = data.as_str() { spawn_quiet("sh", &["-c", &format!("{exec} &")]); }
            s.popup.clear(); s.dirty.store(true, Ordering::Relaxed);
        }
        "power_lock" => { spawn_quiet("loginctl", &["lock-session"]); s.popup.clear(); s.dirty.store(true, Ordering::Relaxed); }
        "power_suspend" => { spawn_quiet("systemctl", &["suspend"]); s.popup.clear(); s.dirty.store(true, Ordering::Relaxed); }
        "power_reboot" => { spawn_quiet("systemctl", &["reboot"]); }
        "power_shutdown" => { spawn_quiet("systemctl", &["poweroff"]); }
        "power_logout" => { spawn_quiet("niri", &["msg", "action", "quit"]); }
        "toggle_night" => { spawn_quiet("sh", &["-c", "pgrep wlsunset && pkill wlsunset || wlsunset -T 4500 -t 3500 &"]); }
        "toggle_bt" => { spawn_quiet("sh", &["-c", "bluetoothctl show | grep -q 'Powered: yes' && bluetoothctl power off || bluetoothctl power on &"]); }
        "toggle_dnd" => { spawn_quiet("sh", &["-c", "makoctl mode -t do-not-disturb"]); }
        "notif_dismiss" => { spawn_quiet("makoctl", &["dismiss", "--all"]); }
        "notif_dismiss_one" => { spawn_quiet("makoctl", &["dismiss"]); }
        // Power profile
        "set_profile" => {
            if let Some(p) = data.as_str() { spawn_quiet("powerprofilesctl", &["set", p]); }
        }
        // Window management
        "close_window" => {
            if let Some(id) = data.as_u64() { spawn_quiet("niri", &["msg", "action", "close-window", "--id", &id.to_string()]); }
        }
        "move_to_workspace" => {
            if let (Some(id), Some(ws)) = (data["id"].as_u64(), data["ws"].as_u64()) {
                spawn_quiet("niri", &["msg", "action", "move-window-to-workspace", "--id", &id.to_string(), &ws.to_string()]);
            }
        }
        // Generic exec (fire-and-forget)
        "exec" => {
            if let Some(cmd) = data.as_str() { spawn_quiet("sh", &["-c", cmd]); }
        }
        // Screenshot
        "screenshot" => { spawn_quiet("sh", &["-c", "grim -g \"$(slurp)\" - | wl-copy &"]); }
        "screenshot_full" => { spawn_quiet("sh", &["-c", "grim - | wl-copy &"]); }
        // Custom state
        "set_custom" => {
            if let (Some(key), value) = (data["key"].as_str(), &data["value"]) {
                s.custom.insert(key.to_string(), value.clone());
                s.dirty.store(true, Ordering::Relaxed);
            }
        }
        "set_theme" => {
            if let Some(t) = data.as_str() {
                s.theme = t.to_string();
                s.dirty.store(true, Ordering::Relaxed);
            }
        }
        "tray_activate" => {
            // data: { "address": "...", "click": "left"|"right" }
            if let (Some(addr), Some(click)) = (data["address"].as_str(), data["click"].as_str()) {
                if let Some(tx) = &s.tray_activate_tx {
                    let _ = tx.try_send((addr.to_string(), click.to_string()));
                }
            }
        }
        _ => eprintln!("[pulpkit] unknown: {c}"),
    }
}

// ── HTML ───────────────────────────────────────────

const DEFAULT_BAR: &str = include_str!("bar.html");
const DEFAULT_POPUP: &str = include_str!("popup.html");

fn load_shell(theme: Option<&str>) -> (String, String, ThemeConfig) {
    if let Some(name) = theme {
        let dir = format!("{}/poc/shells/{}", env!("CARGO_MANIFEST_DIR").trim_end_matches("/poc"), name);
        let bar = std::fs::read_to_string(format!("{dir}/bar.html"))
            .unwrap_or_else(|_| { eprintln!("[pulpkit] no bar.html in {dir}, using default"); DEFAULT_BAR.to_string() });
        let popup = std::fs::read_to_string(format!("{dir}/popup.html"))
            .unwrap_or_else(|_| DEFAULT_POPUP.to_string());
        let config: ThemeConfig = std::fs::read_to_string(format!("{dir}/config.json"))
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        (bar, popup, config)
    } else {
        (DEFAULT_BAR.to_string(), DEFAULT_POPUP.to_string(), ThemeConfig::default())
    }
}

// ── Niri event stream ──────────────────────────────

fn start_tray_watcher(
    tray_items: Arc<std::sync::Mutex<Vec<TrayItem>>>,
    dirty_flag: Arc<AtomicBool>,
    icon_cache: std::collections::HashMap<String, String>,
    activate_rx: tokio::sync::mpsc::Receiver<(String, String)>, // (address, click_type: "left"|"right"|"middle")
) {
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("tokio runtime");
        rt.block_on(async {
            let Ok(client) = system_tray::client::Client::new().await else {
                eprintln!("[pulpkit] tray client failed to start");
                return;
            };
            let mut rx = client.subscribe();
            let mut activate_rx = activate_rx;

            // Build initial items
            {
                let items = client.items().lock().unwrap().clone();
                let mut tray = tray_items.lock().unwrap();
                tray.clear();
                for (addr, (item, menu)) in &items {
                    let id = item.id.clone();
                    let address = addr.clone();
                    let title = item.title.clone().unwrap_or_default();
                    let icon_name = item.icon_name.clone().unwrap_or_default();
                    let icon = if !icon_name.is_empty() {
                        resolve_icon_from_cache(&icon_name, &icon_cache)
                    } else { String::new() };
                    tray.push(TrayItem { id, address, title, icon });
                }
                dirty_flag.store(true, Ordering::Relaxed);
            }

            // Watch for tray updates AND activation requests
            loop {
                tokio::select! {
                    ev = rx.recv() => {
                        if ev.is_err() { break; }
                        let items = client.items().lock().unwrap().clone();
                        let mut tray = tray_items.lock().unwrap();
                        tray.clear();
                        for (addr, (item, _menu)) in &items {
                            let id = item.id.clone();
                            let address = addr.clone();
                            let title = item.title.clone().unwrap_or_default();
                            let icon_name = item.icon_name.clone().unwrap_or_default();
                            let icon = if !icon_name.is_empty() {
                                resolve_icon_from_cache(&icon_name, &icon_cache)
                            } else { String::new() };
                            tray.push(TrayItem { id, address, title, icon });
                        }
                        dirty_flag.store(true, Ordering::Relaxed);
                    }
                    Some((address, click)) = activate_rx.recv() => {
                        use system_tray::client::ActivateRequest;
                        let req = match click.as_str() {
                            "right" => ActivateRequest::Secondary { address, x: 0, y: 0 },
                            _ => ActivateRequest::Default { address, x: 0, y: 0 },
                        };
                        if let Err(e) = client.activate(req).await {
                            eprintln!("[pulpkit] tray activate error: {e}");
                        }
                    }
                }
            }
        });
    });
}

fn resolve_icon_from_cache(name: &str, cache: &std::collections::HashMap<String, String>) -> String {
    cache.get(&name.to_lowercase()).cloned().unwrap_or_else(|| resolve_icon(name))
}

fn start_niri_stream(dirty_flag: Arc<AtomicBool>) {
    std::thread::spawn(move || {
        let Ok(mut child) = Command::new("niri")
            .args(["msg", "event-stream"])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
        else { eprintln!("[pulpkit] niri event-stream failed to start"); return; };

        let stdout = child.stdout.take().unwrap();
        let reader = std::io::BufReader::new(stdout);
        for line in reader.lines() {
            let Ok(line) = line else { break };
            if line.contains("Workspace") {
                dirty_flag.store(true, Ordering::Relaxed);
            }
        }
    });
}

// ── IPC Server ─────────────────────────────────────

type IpcMsg = (String, std::sync::mpsc::Sender<String>);

fn start_ipc_server(
    ipc_tx: std::sync::mpsc::Sender<IpcMsg>,
    dirty: Arc<AtomicBool>,
) {
    let sock_path = format!("{}/pulpkit.sock",
        std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".into()));
    let _ = std::fs::remove_file(&sock_path);

    std::thread::spawn(move || {
        let listener = match UnixListener::bind(&sock_path) {
            Ok(l) => l,
            Err(e) => { eprintln!("[pulpkit] IPC bind failed: {e}"); return; }
        };
        eprintln!("[pulpkit] IPC socket: {sock_path}");

        for stream in listener.incoming().flatten() {
            let tx = ipc_tx.clone();
            std::thread::spawn(move || {
                let reader = std::io::BufReader::new(match stream.try_clone() {
                    Ok(s) => s, Err(_) => return,
                });
                let mut writer = stream;
                for line in reader.lines().flatten() {
                    let trimmed = line.trim().to_string();
                    if trimmed.is_empty() { continue; }
                    if trimmed.starts_with('{') {
                        let (resp_tx, resp_rx) = std::sync::mpsc::channel();
                        if tx.send((trimmed, resp_tx)).is_err() { break; }
                        let response = resp_rx.recv_timeout(std::time::Duration::from_secs(10))
                            .unwrap_or_else(|_| r#"{"ok":false,"error":"timeout"}"#.into());
                        if writeln!(writer, "{response}").is_err() { break; }
                    }
                }
            });
        }
    });
}

// ── Main ───────────────────────────────────────────

fn main() {
    let theme = std::env::args().nth(1);
    let (bar_html, popup_html, theme_config) = load_shell(theme.as_deref());
    let bar_html = Rc::new(bar_html);
    let popup_html = Rc::new(popup_html);
    let theme_config = Rc::new(theme_config);

    let app = Application::builder()
        .application_id("org.pulpkit.webshell")
        .build();

    let bar_html = bar_html.clone();
    let popup_html = popup_html.clone();
    let theme_config = theme_config.clone();
    app.connect_activate(move |app| {
        let bar_height = theme_config.bar_height;
        let popup_width = theme_config.popup_width;
        let popup_height = theme_config.popup_height;
        let bar_bottom = theme_config.bar_position == "bottom";
        let dirty_flag = Arc::new(AtomicBool::new(false));
        let (tray_tx, tray_rx) = tokio::sync::mpsc::channel::<(String, String)>(16);
        let app_state = Rc::new(RefCell::new(AppState {
            popup: String::new(),
            theme: "mocha".into(),
            wifi_nets: Vec::new(),
            apps: Vec::new(),
            tray_items: Vec::new(),
            custom: std::collections::HashMap::new(),
            dirty: dirty_flag.clone(),
            tray_activate_tx: Some(tray_tx),
        }));

        // Cache system info and icons once
        let sys_user = sh("whoami").unwrap_or_default();
        let sys_host = sh("hostname").unwrap_or_default();
        let sys_kernel = sh("uname -r").unwrap_or_default();
        let icon_cache = Rc::new(build_icon_cache());
        eprintln!("[pulpkit] icon cache: {} entries", icon_cache.len());

        // ── BAR WINDOW ──
        let bar_win = gtk4::ApplicationWindow::builder()
            .application(app).default_height(bar_height).build();
        bar_win.init_layer_shell();
        bar_win.set_layer(Layer::Top);
        bar_win.set_keyboard_mode(KeyboardMode::None);
        bar_win.auto_exclusive_zone_enable();
        bar_win.set_anchor(if bar_bottom { Edge::Bottom } else { Edge::Top }, true);
        bar_win.set_anchor(Edge::Left, true);
        bar_win.set_anchor(Edge::Right, true);

        let bar_ucm = UserContentManager::new();
        bar_ucm.register_script_message_handler("pulpkit", None);
        let st = app_state.clone();
        bar_ucm.connect_script_message_received(Some("pulpkit"), move |_ucm, msg| {
            handle_command(msg.to_string().trim().trim_matches('"'), &st);
        });

        let bar_wv = WebView::builder().user_content_manager(&bar_ucm).build();
        bar_wv.set_vexpand(true);
        bar_wv.set_hexpand(true);
        bar_wv.set_background_color(&gtk4::gdk::RGBA::new(0.0, 0.0, 0.0, 0.0));
        bar_wv.load_html(&bar_html, Some("file:///"));
        bar_win.set_child(Some(&bar_wv));
        bar_win.present();

        // ── BACKDROP (click-outside-to-dismiss) ──
        let backdrop_win = gtk4::Window::builder().application(app).build();
        backdrop_win.init_layer_shell();
        backdrop_win.set_layer(Layer::Overlay);
        backdrop_win.set_keyboard_mode(KeyboardMode::None);
        backdrop_win.set_anchor(Edge::Top, true);
        backdrop_win.set_anchor(Edge::Bottom, true);
        backdrop_win.set_anchor(Edge::Left, true);
        backdrop_win.set_anchor(Edge::Right, true);
        // Transparent click-catcher
        let click = gtk4::GestureClick::new();
        let st = app_state.clone();
        click.connect_released(move |_, _, _, _| {
            st.borrow_mut().popup.clear();
            st.borrow().dirty.store(true, Ordering::Relaxed);
        });
        backdrop_win.add_controller(click);
        backdrop_win.set_visible(false);

        // ── POPUP WINDOW (centered) ──
        let popup_win = gtk4::Window::builder()
            .application(app)
            .default_width(popup_width)
            .default_height(popup_height)
            .build();
        popup_win.init_layer_shell();
        popup_win.set_layer(Layer::Overlay);
        popup_win.set_keyboard_mode(KeyboardMode::OnDemand);
        // No anchors = centered by layer-shell

        let popup_ucm = UserContentManager::new();
        popup_ucm.register_script_message_handler("pulpkit", None);
        let st = app_state.clone();
        popup_ucm.connect_script_message_received(Some("pulpkit"), move |_ucm, msg| {
            handle_command(msg.to_string().trim().trim_matches('"'), &st);
        });

        let popup_wv = WebView::builder().user_content_manager(&popup_ucm).build();
        popup_wv.set_vexpand(true);
        popup_wv.set_hexpand(true);
        popup_wv.set_background_color(&gtk4::gdk::RGBA::new(0.0, 0.0, 0.0, 0.0));
        popup_wv.load_html(&popup_html, Some("file:///"));
        popup_win.set_child(Some(&popup_wv));
        popup_win.set_visible(false);

        // ── STATE PUSH ──
        let bar_wv = Rc::new(bar_wv);
        let popup_wv = Rc::new(popup_wv);
        let popup_win = Rc::new(popup_win);
        let backdrop_win = Rc::new(backdrop_win);

        // Start niri event stream
        start_niri_stream(dirty_flag.clone());

        // Start tray watcher
        let tray_shared = Arc::new(std::sync::Mutex::new(Vec::<TrayItem>::new()));
        start_tray_watcher(tray_shared.clone(), dirty_flag.clone(), (*icon_cache).clone(), tray_rx);

        // ── BACKGROUND POLLER THREAD ──
        // All system polling happens here — NEVER on the GTK main thread
        let polled_state = Arc::new(std::sync::Mutex::new(FullState {
            vol: 0, muted: false, audio_device: String::new(),
            bright: 50,
            bat: 100, bat_status: "Unknown".into(), has_bat: has_battery(),
            cpu: 0, mem: 0, disk_used: String::new(), disk_total: String::new(), disk_pct: 0,
            power_profile: "balanced".into(),
            wifi: String::new(), net_signal: 0, net_ip: String::new(), notif_count: 0, dnd: false,
            ws: vec![], windows: vec![], active_title: String::new(), active_app_id: String::new(),
            wifi_nets: vec![], apps: vec![], tray_items: vec![],
            popup: String::new(), theme: "mocha".into(),
            custom: std::collections::HashMap::new(),
            user: sys_user.clone(), host: sys_host.clone(),
            kernel: sys_kernel.clone(), uptime: String::new(),
        }));

        {
            let polled = polled_state.clone();
            let dirty = dirty_flag.clone();
            let icons = (*icon_cache).clone();
            std::thread::spawn(move || {
                loop {
                    let (vol, muted) = poll_vol();
                    let (bat, bat_status) = poll_bat();
                    let (disk_used, disk_total, disk_pct) = poll_disk();
                    let (net_signal, net_ip) = poll_net_details();
                    let (notif_count, dnd) = poll_notifications();
                    let wins = poll_windows(&icons);
                    let active_title = wins.iter().find(|w| w.focused).map(|w| w.title.clone()).unwrap_or_default();
                    let active_app_id = wins.iter().find(|w| w.focused).map(|w| w.app_id.clone()).unwrap_or_default();
                    let s = FullState {
                        vol, muted, audio_device: poll_audio_device(),
                        bright: poll_bri(),
                        bat, bat_status, has_bat: has_battery(),
                        cpu: poll_cpu(), mem: poll_mem(),
                        disk_used, disk_total, disk_pct,
                        power_profile: poll_power_profile(),
                        wifi: poll_wifi(), net_signal, net_ip,
                        notif_count, dnd,
                        ws: poll_ws(), windows: wins,
                        active_title, active_app_id,
                        // Filled in on main thread
                        wifi_nets: vec![], apps: vec![], tray_items: vec![],
                        popup: String::new(), theme: String::new(),
                        custom: std::collections::HashMap::new(),
                        user: String::new(), host: String::new(),
                        kernel: String::new(), uptime: String::new(),
                    };
                    if let Ok(mut p) = polled.lock() { *p = s; }
                    dirty.store(true, Ordering::Relaxed);
                    std::thread::sleep(std::time::Duration::from_millis(1500));
                }
            });
        }

        // ── MAIN THREAD: 80ms timer, only reads shared data + pushes to JS ──
        {
            let bar_wv = bar_wv.clone();
            let popup_wv = popup_wv.clone();
            let popup_win = popup_win.clone();
            let backdrop_win = backdrop_win.clone();
            let app_state = app_state.clone();
            let polled = polled_state.clone();
            let tray_shared = tray_shared.clone();
            let last_json = Rc::new(RefCell::new(String::new()));

            glib::timeout_add_local(std::time::Duration::from_millis(80), move || {
                let dirty = app_state.borrow().dirty.swap(false, Ordering::Relaxed);
                if !dirty { return glib::ControlFlow::Continue; }

                // Sync tray items
                if let Ok(tray) = tray_shared.lock() {
                    app_state.borrow_mut().tray_items = tray.clone();
                }

                // Merge: polled system state + AppState (popup, apps, wifi_nets, tray)
                let as_ = app_state.borrow();
                let mut state = polled.lock().map(|p| p.clone()).unwrap_or_else(|_| FullState {
                    vol: 0, muted: false, audio_device: String::new(),
                    bright: 50,
                    bat: 100, bat_status: "Unknown".into(), has_bat: false,
                    cpu: 0, mem: 0, disk_used: String::new(), disk_total: String::new(), disk_pct: 0,
                    power_profile: "balanced".into(),
                    wifi: String::new(), net_signal: 0, net_ip: String::new(), notif_count: 0, dnd: false,
                    ws: vec![], windows: vec![], active_title: String::new(), active_app_id: String::new(),
                    wifi_nets: vec![], apps: vec![], tray_items: vec![],
                    popup: String::new(), theme: "mocha".into(),
                    custom: std::collections::HashMap::new(),
                    user: String::new(), host: String::new(),
                    kernel: String::new(), uptime: String::new(),
                });
                // Merge AppState fields
                state.popup = as_.popup.clone();
                state.theme = as_.theme.clone();
                state.wifi_nets = as_.wifi_nets.clone();
                state.apps = as_.apps.clone();
                state.tray_items = as_.tray_items.clone();
                state.custom = as_.custom.clone();
                state.user = sys_user.clone();
                state.host = sys_host.clone();
                state.kernel = sys_kernel.clone();
                drop(as_);

                let json = serde_json::to_string(&state).unwrap_or_else(|_| "{}".into());
                let mut last = last_json.borrow_mut();
                if json == *last { return glib::ControlFlow::Continue; }
                *last = json.clone();
                drop(last);

                let script = format!("if(typeof updateState==='function')updateState({json})");
                bar_wv.evaluate_javascript(&script, None, None, None::<&gtk4::gio::Cancellable>, |_| {});

                if state.popup.is_empty() {
                    backdrop_win.set_visible(false);
                    popup_win.set_visible(false);
                } else {
                    backdrop_win.set_visible(true);
                    popup_win.set_visible(true);
                    popup_wv.evaluate_javascript(&script, None, None, None::<&gtk4::gio::Cancellable>, |_| {});
                }

                glib::ControlFlow::Continue
            });
        }

        // ── IPC SERVER ──
        let (ipc_tx, ipc_rx) = std::sync::mpsc::channel::<IpcMsg>();
        start_ipc_server(ipc_tx, dirty_flag.clone());

        // ── IPC MESSAGE PROCESSOR (50ms timer on main thread) ──
        {
            let bar_wv = bar_wv.clone();
            let popup_wv = popup_wv.clone();
            let popup_win = popup_win.clone();
            let backdrop_win = backdrop_win.clone();
            let app_state = app_state.clone();
            let polled = polled_state.clone();
            let sys_user = sh("whoami").unwrap_or_default();
            let sys_host = sh("hostname").unwrap_or_default();
            let sys_kernel = sh("uname -r").unwrap_or_default();

            glib::timeout_add_local(std::time::Duration::from_millis(50), move || {
                // Drain all pending IPC messages
                while let Ok((request_json, resp_tx)) = ipc_rx.try_recv() {
                    let parsed: serde_json::Value = serde_json::from_str(&request_json).unwrap_or_default();
                    let method = parsed["method"].as_str().unwrap_or("");

                    let response: String = match method {
                        "get_state" => {
                            let as_ = app_state.borrow();
                            let mut state = polled.lock().map(|p| p.clone()).unwrap_or_default();
                            state.popup = as_.popup.clone();
                            state.theme = as_.theme.clone();
                            state.wifi_nets = as_.wifi_nets.clone();
                            state.apps = as_.apps.clone();
                            state.tray_items = as_.tray_items.clone();
                            state.custom = as_.custom.clone();
                            state.user = sys_user.clone();
                            state.host = sys_host.clone();
                            state.kernel = sys_kernel.clone();
                            drop(as_);
                            serde_json::to_string(&serde_json::json!({"ok": true, "data": state}))
                                .unwrap_or_else(|_| r#"{"ok":false}"#.into())
                        }
                        "reload_bar" => {
                            if let Some(path) = parsed["path"].as_str() {
                                let uri = format!("file://{path}");
                                bar_wv.load_uri(&uri);
                                r#"{"ok":true}"#.into()
                            } else if let Some(html) = parsed["html"].as_str() {
                                bar_wv.load_html(html, Some("file:///"));
                                r#"{"ok":true}"#.into()
                            } else { r#"{"ok":false,"error":"missing html or path"}"#.into() }
                        }
                        "reload_popup" => {
                            if let Some(path) = parsed["path"].as_str() {
                                let uri = format!("file://{path}");
                                popup_wv.load_uri(&uri);
                                r#"{"ok":true}"#.into()
                            } else if let Some(html) = parsed["html"].as_str() {
                                popup_wv.load_html(html, Some("file:///"));
                                r#"{"ok":true}"#.into()
                            } else { r#"{"ok":false,"error":"missing html or path"}"#.into() }
                        }
                        "eval_js" => {
                            let target = parsed["target"].as_str().unwrap_or("bar");
                            let script = parsed["script"].as_str().unwrap_or("");
                            let wv = if target == "popup" { &*popup_wv } else { &*bar_wv };
                            wv.evaluate_javascript(script, None, None, None::<&gtk4::gio::Cancellable>, |_| {});
                            r#"{"ok":true}"#.into()
                        }
                        "set_mock_state" => {
                            if let Some(data) = parsed.get("data") {
                                let script = format!("if(typeof updateState==='function')updateState({})", data);
                                bar_wv.evaluate_javascript(&script, None, None, None::<&gtk4::gio::Cancellable>, |_| {});
                                popup_win.set_visible(true);
                                backdrop_win.set_visible(true);
                                popup_wv.evaluate_javascript(&script, None, None, None::<&gtk4::gio::Cancellable>, |_| {});
                                r#"{"ok":true}"#.into()
                            } else { r#"{"ok":false,"error":"missing data"}"#.into() }
                        }
                        "get_console_logs" => {
                            r#"{"ok":true,"data":"[]"}"#.into()
                        }
                        _ => format!(r#"{{"ok":false,"error":"unknown method: {method}"}}"#),
                    };
                    let _ = resp_tx.send(response);
                }
                glib::ControlFlow::Continue
            });
        }

        eprintln!("[pulpkit] shell running");
    });

    let empty: Vec<String> = vec![];
    app.run_with_args(&empty);
}
