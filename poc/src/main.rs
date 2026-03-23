pub mod greetd;
mod icons;
mod ipc;
pub mod pam;
mod poll;
mod state;
mod watchers;

use std::cell::RefCell;
use std::process::{Command, Stdio};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use gtk4::glib;
use gtk4::prelude::*;
use gtk4::Application;
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
use webkit6::prelude::*;
use webkit6::{UserContentManager, WebView};

use icons::build_icon_cache;
use ipc::{start_ipc_server, IpcMsg};
use poll::*;
use state::*;

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
    bar_position: String,
    #[serde(default = "default_popup_backdrop")]
    popup_backdrop: String,
    #[serde(default = "default_popup_dim_opacity")]
    popup_dim_opacity: f32,
}

fn default_bar_height() -> i32 { DEFAULT_BAR_HEIGHT }
fn default_popup_width() -> i32 { DEFAULT_POPUP_WIDTH }
fn default_popup_height() -> i32 { DEFAULT_POPUP_HEIGHT }
fn default_popup_backdrop() -> String { "dim".into() }
fn default_popup_dim_opacity() -> f32 { 0.25 }

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            bar_height: DEFAULT_BAR_HEIGHT, popup_width: DEFAULT_POPUP_WIDTH,
            popup_height: DEFAULT_POPUP_HEIGHT, bar_position: "top".into(),
            popup_backdrop: "dim".into(), popup_dim_opacity: 0.25,
        }
    }
}

// ── Helpers ────────────────────────────────────────

fn spawn_quiet(cmd: &str, args: &[&str]) {
    let _ = Command::new(cmd).args(args)
        .stdout(Stdio::null()).stderr(Stdio::null()).spawn();
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
        "set_profile" => {
            if let Some(p) = data.as_str() { spawn_quiet("powerprofilesctl", &["set", p]); }
        }
        "close_window" => {
            if let Some(id) = data.as_u64() { spawn_quiet("niri", &["msg", "action", "close-window", "--id", &id.to_string()]); }
        }
        "move_to_workspace" => {
            if let (Some(id), Some(ws)) = (data["id"].as_u64(), data["ws"].as_u64()) {
                spawn_quiet("niri", &["msg", "action", "move-window-to-workspace", "--id", &id.to_string(), &ws.to_string()]);
            }
        }
        "exec" => {
            if let Some(cmd) = data.as_str() { spawn_quiet("sh", &["-c", cmd]); }
        }
        "screenshot" => { spawn_quiet("sh", &["-c", "grim -g \"$(slurp)\" - | wl-copy &"]); }
        "screenshot_full" => { spawn_quiet("sh", &["-c", "grim - | wl-copy &"]); }
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
            if let (Some(addr), Some(click)) = (data["address"].as_str(), data["click"].as_str()) {
                if let Some(tx) = &s.tray_activate_tx {
                    let _ = tx.try_send((addr.to_string(), click.to_string()));
                }
            }
        }
        "verify_password" => {
            if let Some(password) = data.as_str() {
                let ok = crate::pam::verify_password(password);
                s.custom.insert("auth_result".to_string(), serde_json::Value::Bool(ok));
                s.dirty.store(true, Ordering::Relaxed);
            }
        }
        "unlock" => {
            spawn_quiet("loginctl", &["unlock-session"]);
            s.custom.remove("auth_result");
            s.dirty.store(true, Ordering::Relaxed);
        }
        "notif_clear_all" => {
            s.clear_notifications = true;
            s.dirty.store(true, Ordering::Relaxed);
        }
        _ => eprintln!("[pulpkit] unknown: {c}"),
    }
}

// ── HTML ───────────────────────────────────────────

const DEFAULT_BAR: &str = include_str!("bar.html");
const DEFAULT_POPUP: &str = include_str!("popup.html");
const DEFAULT_TOAST: &str = "<html><body></body></html>";

fn load_shell(theme: Option<&str>) -> (String, String, String, ThemeConfig) {
    if let Some(name) = theme {
        let dir = format!("{}/poc/shells/{}", env!("CARGO_MANIFEST_DIR").trim_end_matches("/poc"), name);
        let bar = std::fs::read_to_string(format!("{dir}/bar.html"))
            .unwrap_or_else(|_| { eprintln!("[pulpkit] no bar.html in {dir}, using default"); DEFAULT_BAR.to_string() });
        let popup = std::fs::read_to_string(format!("{dir}/popup.html"))
            .unwrap_or_else(|_| DEFAULT_POPUP.to_string());
        let toast = std::fs::read_to_string(format!("{dir}/toast.html"))
            .unwrap_or_else(|_| DEFAULT_TOAST.to_string());
        let config: ThemeConfig = std::fs::read_to_string(format!("{dir}/config.json"))
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        (bar, popup, toast, config)
    } else {
        (DEFAULT_BAR.to_string(), DEFAULT_POPUP.to_string(), DEFAULT_TOAST.to_string(), ThemeConfig::default())
    }
}

// ── Main ───────────────────────────────────────────

fn main() {
    let theme = std::env::args().nth(1);
    let (bar_html, popup_html, toast_html, theme_config) = load_shell(theme.as_deref());
    let bar_html = Rc::new(bar_html);
    let popup_html = Rc::new(popup_html);
    let toast_html = Rc::new(toast_html);
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
            clear_notifications: false,
        }));

        // Cache system info and icons once
        let sys_user = sh("whoami").unwrap_or_default();
        let sys_host = sh("hostname").unwrap_or_default();
        let sys_kernel = sh("uname -r").unwrap_or_default();
        let icon_cache = Rc::new(build_icon_cache());
        eprintln!("[pulpkit] icon cache: {} entries", icon_cache.len());

        // ── GLOBAL: Make all windows transparent ──
        let global_css = gtk4::CssProvider::new();
        global_css.load_from_data("window { background: transparent; }");
        gtk4::style_context_add_provider_for_display(
            &gtk4::gdk::Display::default().unwrap(),
            &global_css,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );

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

        // ── BACKDROP ──
        let backdrop_win = gtk4::Window::builder().application(app).build();
        backdrop_win.init_layer_shell();
        backdrop_win.set_layer(Layer::Overlay);
        backdrop_win.set_keyboard_mode(KeyboardMode::None);
        backdrop_win.set_anchor(Edge::Top, true);
        backdrop_win.set_anchor(Edge::Bottom, true);
        backdrop_win.set_anchor(Edge::Left, true);
        backdrop_win.set_anchor(Edge::Right, true);
        let click = gtk4::GestureClick::new();
        let st = app_state.clone();
        click.connect_released(move |_, _, _, _| {
            st.borrow_mut().popup.clear();
            st.borrow().dirty.store(true, Ordering::Relaxed);
        });
        backdrop_win.add_controller(click);
        // Apply configurable backdrop opacity
        let backdrop_css = gtk4::CssProvider::new();
        let dim_opacity = theme_config.popup_dim_opacity;
        let css_str = match theme_config.popup_backdrop.as_str() {
            "none" => "window.backdrop { background: transparent; }".to_string(),
            "dim" => format!("window.backdrop {{ background: rgba(0,0,0,{dim_opacity}); }}"),
            _ => "window.backdrop { background: rgba(0,0,0,0.95); }".to_string(),
        };
        backdrop_css.load_from_data(&css_str);
        backdrop_win.add_css_class("backdrop");
        gtk4::style_context_add_provider_for_display(
            &gtk4::gdk::Display::default().unwrap(),
            &backdrop_css,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
        backdrop_win.set_visible(false);

        // ── POPUP WINDOW ──
        let popup_win = gtk4::Window::builder()
            .application(app)
            .default_width(popup_width)
            .default_height(popup_height)
            .build();
        popup_win.init_layer_shell();
        popup_win.set_layer(Layer::Overlay);
        popup_win.set_keyboard_mode(KeyboardMode::OnDemand);

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

        // ── TOAST NOTIFICATION WINDOW ──
        let toast_win = gtk4::Window::builder()
            .application(app)
            .default_width(380)
            .default_height(300)
            .build();
        toast_win.init_layer_shell();
        toast_win.set_layer(Layer::Top);
        toast_win.set_keyboard_mode(KeyboardMode::None);
        toast_win.set_anchor(Edge::Top, true);
        toast_win.set_anchor(Edge::Right, true);
        toast_win.set_margin(Edge::Top, 2);
        toast_win.set_margin(Edge::Right, 8);

        let toast_ucm = UserContentManager::new();
        toast_ucm.register_script_message_handler("pulpkit", None);
        let st = app_state.clone();
        toast_ucm.connect_script_message_received(Some("pulpkit"), move |_ucm, msg| {
            handle_command(msg.to_string().trim().trim_matches('"'), &st);
        });

        let toast_wv = WebView::builder().user_content_manager(&toast_ucm).build();
        toast_wv.set_vexpand(true);
        toast_wv.set_hexpand(true);
        toast_wv.set_background_color(&gtk4::gdk::RGBA::new(0.0, 0.0, 0.0, 0.0));
        toast_wv.load_html(&toast_html, Some("file:///"));
        toast_win.set_child(Some(&toast_wv));
        toast_win.present();

        // ── STATE PUSH ──
        let bar_wv = Rc::new(bar_wv);
        let toast_wv = Rc::new(toast_wv);
        let popup_wv = Rc::new(popup_wv);
        let popup_win = Rc::new(popup_win);
        let backdrop_win = Rc::new(backdrop_win);

        let polled_state = Arc::new(std::sync::Mutex::new(FullState {
            has_bat: has_battery(),
            ..Default::default()
        }));

        // ── START REACTIVE WATCHERS ──
        watchers::niri::start_niri_stream(polled_state.clone(), dirty_flag.clone(), (*icon_cache).clone());

        let tray_shared = Arc::new(std::sync::Mutex::new(Vec::<TrayItem>::new()));
        watchers::tray::start_tray_watcher(tray_shared.clone(), dirty_flag.clone(), (*icon_cache).clone(), tray_rx);

        watchers::audio::start_audio_watcher(polled_state.clone(), dirty_flag.clone());
        watchers::upower::start_upower_watcher(polled_state.clone(), dirty_flag.clone());
        watchers::network::start_network_watcher(polled_state.clone(), dirty_flag.clone());
        watchers::bluetooth::start_bluetooth_watcher(polled_state.clone(), dirty_flag.clone());
        watchers::mpris::start_mpris_watcher(polled_state.clone(), dirty_flag.clone());
        watchers::clipboard::start_clipboard_watcher(polled_state.clone(), dirty_flag.clone());
        watchers::power_profiles::start_power_profiles_watcher(polled_state.clone(), dirty_flag.clone());
        watchers::brightness::start_brightness_watcher(polled_state.clone(), dirty_flag.clone());
        watchers::logind::start_logind_watcher(polled_state.clone(), dirty_flag.clone());
        watchers::gamemode::start_gamemode_watcher(polled_state.clone(), dirty_flag.clone());
        watchers::gpu::start_gpu_watcher(polled_state.clone(), dirty_flag.clone());
        watchers::discord::start_discord_watcher(polled_state.clone(), dirty_flag.clone());
        watchers::gamescope::start_gamescope_watcher(polled_state.clone(), dirty_flag.clone());
        watchers::notifications::start_notification_daemon(polled_state.clone(), dirty_flag.clone());
        watchers::keyboard::start_keyboard_watcher(polled_state.clone(), dirty_flag.clone());
        watchers::udisks::start_udisks_watcher(polled_state.clone(), dirty_flag.clone());
        watchers::night_light::start_night_light_watcher(polled_state.clone(), dirty_flag.clone());
        watchers::net_speed::start_net_speed_watcher(polled_state.clone(), dirty_flag.clone());
        watchers::inhibitors::start_inhibitor_watcher(polled_state.clone(), dirty_flag.clone());
        watchers::thermal::start_thermal_watcher(polled_state.clone(), dirty_flag.clone());
        watchers::ac_power::start_ac_power_watcher(polled_state.clone(), dirty_flag.clone());
        watchers::dbus_service::start_dbus_service(polled_state.clone(), dirty_flag.clone());
        watchers::audio_streams::start_audio_streams_watcher(polled_state.clone(), dirty_flag.clone());
        watchers::calendar::start_calendar_watcher(polled_state.clone(), dirty_flag.clone());
        watchers::compositor::start_compositor_watcher(polled_state.clone(), dirty_flag.clone());
        watchers::input_method::start_input_method_watcher(polled_state.clone(), dirty_flag.clone());
        watchers::load_avg::start_load_avg_watcher(polled_state.clone(), dirty_flag.clone());
        watchers::outputs::start_outputs_watcher(polled_state.clone(), dirty_flag.clone());
        watchers::polkit::start_polkit_watcher(polled_state.clone(), dirty_flag.clone());
        watchers::recent_files::start_recent_files_watcher(polled_state.clone(), dirty_flag.clone());
        watchers::swap::start_swap_watcher(polled_state.clone(), dirty_flag.clone());
        watchers::weather::start_weather_watcher(polled_state.clone(), dirty_flag.clone());
        watchers::systemd::start_systemd_watcher(polled_state.clone(), dirty_flag.clone());
        watchers::vpn::start_vpn_watcher(polled_state.clone(), dirty_flag.clone());
        watchers::containers::start_containers_watcher(polled_state.clone(), dirty_flag.clone());
        watchers::fan::start_fan_watcher(polled_state.clone(), dirty_flag.clone());
        watchers::power_draw::start_power_draw_watcher(polled_state.clone(), dirty_flag.clone());
        watchers::sunrise::start_sunrise_watcher(polled_state.clone(), dirty_flag.clone());
        watchers::systemd_timers::start_systemd_timers_watcher(polled_state.clone(), dirty_flag.clone());
        watchers::journal::start_journal_watcher(polled_state.clone(), dirty_flag.clone());
        watchers::ssh_sessions::start_ssh_sessions_watcher(polled_state.clone(), dirty_flag.clone());
        watchers::focus_tracker::start_focus_tracker(polled_state.clone(), dirty_flag.clone());
        watchers::caffeine::start_caffeine_watcher(polled_state.clone(), dirty_flag.clone());
        watchers::mic::start_mic_watcher(polled_state.clone(), dirty_flag.clone());
        watchers::audio_devices::start_audio_devices_watcher(polled_state.clone(), dirty_flag.clone());
        watchers::packagekit::start_packagekit_watcher(polled_state.clone(), dirty_flag.clone());
        watchers::timezone::start_timezone_watcher(polled_state.clone(), dirty_flag.clone());
        watchers::screen_share::start_screen_share_watcher(polled_state.clone(), dirty_flag.clone());
        watchers::trash::start_trash_watcher(polled_state.clone(), dirty_flag.clone());
        watchers::top_procs::start_top_procs_watcher(polled_state.clone(), dirty_flag.clone());
        watchers::user_info::start_user_info_watcher(polled_state.clone(), dirty_flag.clone());

        // ── BACKGROUND POLLER THREAD ──
        // Only cpu, mem, disk, uptime remain — everything else is reactive.
        {
            let polled = polled_state.clone();
            let dirty = dirty_flag.clone();
            std::thread::spawn(move || {
                loop {
                    let (disk_used, disk_total, disk_pct) = poll_disk();
                    let uptime = sh("uptime -p").unwrap_or_default();
                    if let Ok(mut s) = polled.lock() {
                        s.cpu = poll_cpu();
                        s.mem = poll_mem();
                        s.disk_used = disk_used;
                        s.disk_total = disk_total;
                        s.disk_pct = disk_pct;
                        s.uptime = uptime;
                    }
                    dirty.store(true, Ordering::Relaxed);
                    std::thread::sleep(std::time::Duration::from_secs(3));
                }
            });
        }

        // ── MAIN THREAD: 80ms timer ──
        {
            let bar_wv = bar_wv.clone();
            let popup_wv = popup_wv.clone();
            let popup_win = popup_win.clone();
            let backdrop_win = backdrop_win.clone();
            let app_state = app_state.clone();
            let use_backdrop = theme_config.popup_backdrop != "none";
            let toast_wv = toast_wv.clone();
            let polled = polled_state.clone();
            let tray_shared = tray_shared.clone();
            let last_json = Rc::new(RefCell::new(String::new()));

            glib::timeout_add_local(std::time::Duration::from_millis(80), move || {
                let dirty = app_state.borrow().dirty.swap(false, Ordering::Relaxed);
                if !dirty { return glib::ControlFlow::Continue; }

                if let Ok(tray) = tray_shared.lock() {
                    app_state.borrow_mut().tray_items = tray.clone();
                }

                // Handle clear notifications request
                if app_state.borrow().clear_notifications {
                    if let Ok(mut ps) = polled.lock() {
                        ps.notifications.clear();
                        ps.notif_count = 0;
                    }
                    app_state.borrow_mut().clear_notifications = false;
                }

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

                let json = serde_json::to_string(&state).unwrap_or_else(|_| "{}".into());
                let mut last = last_json.borrow_mut();
                if json == *last { return glib::ControlFlow::Continue; }
                *last = json.clone();
                drop(last);

                let script = format!("if(typeof updateState==='function')updateState({json})");
                bar_wv.evaluate_javascript(&script, None, None, None::<&gtk4::gio::Cancellable>, |_| {});
                toast_wv.evaluate_javascript(&script, None, None, None::<&gtk4::gio::Cancellable>, |_| {});

                if state.popup.is_empty() {
                    if use_backdrop { backdrop_win.set_visible(false); }
                    popup_win.set_visible(false);
                } else {
                    if use_backdrop { backdrop_win.set_visible(true); }
                    popup_win.set_visible(true);
                    popup_wv.evaluate_javascript(&script, None, None, None::<&gtk4::gio::Cancellable>, |_| {});
                }

                glib::ControlFlow::Continue
            });
        }

        // ── IPC SERVER ──
        let (ipc_tx, ipc_rx) = std::sync::mpsc::channel::<IpcMsg>();
        start_ipc_server(ipc_tx, dirty_flag.clone());

        // ── IPC MESSAGE PROCESSOR ──
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
                                bar_wv.load_uri(&format!("file://{path}"));
                                r#"{"ok":true}"#.into()
                            } else if let Some(html) = parsed["html"].as_str() {
                                bar_wv.load_html(html, Some("file:///"));
                                r#"{"ok":true}"#.into()
                            } else { r#"{"ok":false,"error":"missing html or path"}"#.into() }
                        }
                        "reload_popup" => {
                            if let Some(path) = parsed["path"].as_str() {
                                popup_wv.load_uri(&format!("file://{path}"));
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
