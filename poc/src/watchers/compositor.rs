use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use crate::state::FullState;

pub fn start_compositor_watcher(state: Arc<Mutex<FullState>>, dirty: Arc<AtomicBool>) {
    std::thread::spawn(move || {
        let compositor = detect_compositor();
        if let Ok(mut s) = state.lock() {
            s.compositor = compositor;
        }
        dirty.store(true, Ordering::Relaxed);
        // One-shot: no loop needed
    });
}

fn detect_compositor() -> String {
    if std::env::var("NIRI_SOCKET").is_ok() {
        return "niri".to_string();
    }
    if std::env::var("HYPRLAND_INSTANCE_SIGNATURE").is_ok() {
        return "hyprland".to_string();
    }
    if std::env::var("SWAYSOCK").is_ok() {
        return "sway".to_string();
    }
    // Fallback: check XDG_CURRENT_DESKTOP
    if let Ok(desktop) = std::env::var("XDG_CURRENT_DESKTOP") {
        let lower = desktop.to_lowercase();
        if lower.contains("niri") {
            return "niri".to_string();
        }
        if lower.contains("hyprland") {
            return "hyprland".to_string();
        }
        if lower.contains("sway") {
            return "sway".to_string();
        }
        if lower.contains("gnome") {
            return "gnome".to_string();
        }
        if lower.contains("kde") || lower.contains("plasma") {
            return "kde".to_string();
        }
        return lower;
    }
    "unknown".to_string()
}
