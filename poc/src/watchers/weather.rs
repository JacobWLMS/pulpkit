use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use crate::poll::sh;
use crate::state::FullState;

pub fn start_weather_watcher(state: Arc<Mutex<FullState>>, dirty: Arc<AtomicBool>) {
    std::thread::spawn(move || loop {
        if let Some(weather) = fetch_weather() {
            if let Ok(mut s) = state.lock() {
                s.weather_temp = weather.0;
                s.weather_condition = weather.1;
                s.weather_icon = weather.2;
            }
            dirty.store(true, Ordering::Relaxed);
        }
        std::thread::sleep(std::time::Duration::from_secs(900));
    });
}

fn fetch_weather() -> Option<(f32, String, String)> {
    let output = sh("curl -s 'wttr.in/?format=%t|%C|%c' 2>/dev/null")?;
    let parts: Vec<&str> = output.splitn(3, '|').collect();
    if parts.len() < 3 {
        return None;
    }
    // Temperature comes as e.g. "+12°C" or "-3°F" — strip non-numeric prefix/suffix
    let temp_str = parts[0]
        .trim()
        .replace("°C", "")
        .replace("°F", "")
        .replace("+", "");
    let temp = temp_str.trim().parse::<f32>().unwrap_or(0.0);
    let condition = parts[1].trim().to_string();
    let icon = parts[2].trim().to_string();
    Some((temp, condition, icon))
}
