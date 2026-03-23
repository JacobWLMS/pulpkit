use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use serde::Serialize;

// ── State structs ──────────────────────────────────

#[derive(Serialize, Clone)]
pub struct Workspace {
    pub idx: u32,
    pub active: bool,
}

#[derive(Serialize, Clone)]
pub struct WindowInfo {
    pub id: u64,
    pub title: String,
    pub app_id: String,
    pub focused: bool,
    pub icon: String,
}

#[derive(Serialize, Clone)]
pub struct WifiNet {
    pub ssid: String,
    pub signal: u32,
    pub secure: bool,
    pub active: bool,
}

#[derive(Serialize, Clone)]
pub struct AppEntry {
    pub name: String,
    pub exec: String,
    pub icon: String,
}

#[derive(Serialize, Clone)]
pub struct TrayItem {
    pub id: String,
    pub address: String,
    pub title: String,
    pub icon: String,
}

#[derive(Serialize, Clone)]
pub struct BtDevice {
    pub name: String,
    pub address: String,
    pub connected: bool,
    pub icon: String,
}

#[derive(Serialize, Clone, Default)]
pub struct FullState {
    // Audio
    pub vol: u32,
    pub muted: bool,
    pub audio_device: String,
    // Display
    pub bright: u32,
    // Battery
    pub bat: u32,
    pub bat_status: String,
    pub has_bat: bool,
    // System
    pub cpu: u32,
    pub mem: u32,
    pub disk_used: String,
    pub disk_total: String,
    pub disk_pct: u32,
    pub power_profile: String,
    // Network
    pub wifi: String,
    pub net_signal: u32,
    pub net_ip: String,
    // Notifications
    pub notif_count: u32,
    pub dnd: bool,
    // Workspaces & windows
    pub ws: Vec<Workspace>,
    pub windows: Vec<WindowInfo>,
    pub active_title: String,
    pub active_app_id: String,
    // Dynamic lists (populated on demand)
    pub wifi_nets: Vec<WifiNet>,
    pub apps: Vec<AppEntry>,
    pub tray_items: Vec<TrayItem>,
    // UI state
    pub popup: String,
    pub theme: String,
    // Custom key-value store
    pub custom: std::collections::HashMap<String, serde_json::Value>,
    // Static system info
    pub user: String,
    pub host: String,
    pub kernel: String,
    pub uptime: String,
    // Bluetooth
    pub bt_powered: bool,
    pub bt_connected: Vec<BtDevice>,
    // MPRIS media
    pub media_playing: bool,
    pub media_title: String,
    pub media_artist: String,
    pub media_album: String,
    pub media_art_url: String,
    pub media_player: String,
    // Clipboard
    pub clipboard_text: String,
    // Session (logind)
    pub session_locked: bool,
    pub session_idle: bool,
    pub preparing_sleep: bool,
    // Gaming
    pub gamemode_active: bool,
    pub gpu_usage: u32,
    pub gpu_temp: u32,
    pub vram_used_mb: u32,
    pub vram_total_mb: u32,
    pub gamescope_active: bool,
    // Discord
    pub discord_activity: String,
    // Notifications (daemon-managed)
    pub notifications: Vec<Notification>,
    // Keyboard
    pub kb_layout: String,
    pub kb_variant: String,
    // Removable drives
    pub drives: Vec<DriveInfo>,
    // Night light
    pub night_light_active: bool,
    // Network speed
    pub net_rx_bytes_sec: u64,
    pub net_tx_bytes_sec: u64,
    // Idle inhibitors
    pub inhibitors: Vec<InhibitorInfo>,
    // Thermal
    pub cpu_temp: u32,
    // Power supply
    pub ac_plugged: bool,
    // Systemd services
    pub failed_units: Vec<String>,
    pub failed_unit_count: u32,
    // Audio devices
    pub audio_sinks: Vec<AudioDevice>,
    pub audio_sources: Vec<AudioDevice>,
    // Package updates
    pub updates_available: u32,
    // Timezone
    pub timezone: String,
    // Screen sharing
    pub screen_sharing: bool,
    // Trash
    pub trash_count: u32,
    // Top processes
    pub top_procs: Vec<ProcessInfo>,
    // User
    pub user_icon: String,
    // Display outputs
    pub outputs: Vec<DisplayOutput>,
    // Polkit
    pub polkit_pending: bool,
    pub polkit_message: String,
    // Fcitx/IBus input method
    pub im_active: bool,
    pub im_name: String,
    // Pipewire audio streams
    pub audio_streams: Vec<AudioStream>,
    // Recent files
    pub recent_files: Vec<RecentFile>,
    // Calendar events (ical)
    pub calendar_events: Vec<CalendarEvent>,
    // Weather
    pub weather_temp: f32,
    pub weather_condition: String,
    pub weather_icon: String,
    // Swap
    pub swap_used_mb: u32,
    pub swap_total_mb: u32,
    // Load average
    pub load_1: f32,
    pub load_5: f32,
    pub load_15: f32,
    // Sway/Hyprland compat
    pub compositor: String,
    // VPN
    pub vpn_active: bool,
    pub vpn_name: String,
    // Docker/Podman containers
    pub containers: Vec<ContainerInfo>,
    // Fan speed
    pub fan_rpm: u32,
    // Power draw
    pub power_draw_watts: f32,
    // Sunrise/sunset
    pub sunrise: String,
    pub sunset: String,
    // Systemd timers
    pub timers: Vec<TimerInfo>,
    // Journal (recent entries)
    pub journal_errors: Vec<JournalEntry>,
    // SSH sessions
    pub ssh_sessions: u32,
    // App focus tracking
    pub focused_app_time_secs: u32,
    // Caffeine (manual idle inhibit)
    pub caffeine_active: bool,
    // Microphone
    pub mic_muted: bool,
    pub mic_volume: u32,
}

#[derive(Serialize, Clone)]
pub struct ContainerInfo {
    pub name: String,
    pub image: String,
    pub status: String,
    pub id: String,
}

#[derive(Serialize, Clone)]
pub struct TimerInfo {
    pub name: String,
    pub next_trigger: String,
    pub last_trigger: String,
}

#[derive(Serialize, Clone)]
pub struct JournalEntry {
    pub unit: String,
    pub message: String,
    pub priority: u32,
    pub timestamp: String,
}

#[derive(Serialize, Clone)]
pub struct DisplayOutput {
    pub name: String,
    pub make: String,
    pub model: String,
    pub width: u32,
    pub height: u32,
    pub refresh: f32,
    pub scale: f32,
    pub enabled: bool,
}

#[derive(Serialize, Clone)]
pub struct AudioStream {
    pub name: String,
    pub app_name: String,
    pub volume: u32,
    pub muted: bool,
    pub is_input: bool,
}

#[derive(Serialize, Clone)]
pub struct RecentFile {
    pub name: String,
    pub uri: String,
    pub mime_type: String,
    pub timestamp: u64,
}

#[derive(Serialize, Clone)]
pub struct CalendarEvent {
    pub summary: String,
    pub start: String,
    pub end: String,
    pub location: String,
}

#[derive(Serialize, Clone)]
pub struct AudioDevice {
    pub name: String,
    pub description: String,
    pub active: bool,
    pub volume: u32,
    pub muted: bool,
}

#[derive(Serialize, Clone)]
pub struct ProcessInfo {
    pub name: String,
    pub pid: u32,
    pub cpu_pct: f32,
    pub mem_mb: u32,
}

#[derive(Serialize, Clone)]
pub struct DriveInfo {
    pub name: String,
    pub mount_point: String,
    pub size_bytes: u64,
    pub device: String,
}

#[derive(Serialize, Clone)]
pub struct InhibitorInfo {
    pub who: String,
    pub why: String,
    pub what: String,
}

#[derive(Serialize, Clone)]
pub struct Notification {
    pub id: u32,
    pub app_name: String,
    pub summary: String,
    pub body: String,
    pub icon: String,
    pub timestamp: u64,
}

pub struct AppState {
    pub popup: String,
    pub theme: String,
    pub wifi_nets: Vec<WifiNet>,
    pub apps: Vec<AppEntry>,
    pub tray_items: Vec<TrayItem>,
    pub custom: std::collections::HashMap<String, serde_json::Value>,
    pub dirty: Arc<AtomicBool>,
    pub tray_activate_tx: Option<tokio::sync::mpsc::Sender<(String, String)>>,
    pub clear_notifications: bool,
}
