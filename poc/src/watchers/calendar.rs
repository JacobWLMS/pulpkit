use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use crate::state::{CalendarEvent, FullState};

pub fn start_calendar_watcher(state: Arc<Mutex<FullState>>, dirty: Arc<AtomicBool>) {
    std::thread::spawn(move || loop {
        let events = read_calendar_events();
        if let Ok(mut s) = state.lock() {
            s.calendar_events = events;
        }
        dirty.store(true, Ordering::Relaxed);
        std::thread::sleep(std::time::Duration::from_secs(300));
    });
}

fn find_calendar_dir() -> Option<String> {
    let home = std::env::var("HOME").unwrap_or_default();
    let candidates = [
        format!("{home}/.local/share/evolution/calendar"),
        format!("{home}/.calendars"),
    ];
    for dir in &candidates {
        if std::path::Path::new(dir).is_dir() {
            return Some(dir.clone());
        }
    }
    None
}

fn read_calendar_events() -> Vec<CalendarEvent> {
    let Some(dir) = find_calendar_dir() else {
        return Vec::new();
    };
    let mut events = Vec::new();
    collect_ics_files(&dir, &mut events);
    events
}

fn collect_ics_files(dir: &str, events: &mut Vec<CalendarEvent>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_ics_files(&path.to_string_lossy(), events);
        } else if path.extension().is_some_and(|e| e == "ics") {
            if let Ok(content) = std::fs::read_to_string(&path) {
                parse_ics(&content, events);
            }
        }
    }
}

fn parse_ics(content: &str, events: &mut Vec<CalendarEvent>) {
    let mut in_vevent = false;
    let mut summary = String::new();
    let mut dtstart = String::new();
    let mut dtend = String::new();
    let mut location = String::new();

    for line in content.lines() {
        let line = line.trim_end_matches('\r');
        if line == "BEGIN:VEVENT" {
            in_vevent = true;
            summary.clear();
            dtstart.clear();
            dtend.clear();
            location.clear();
        } else if line == "END:VEVENT" {
            if in_vevent && !summary.is_empty() {
                events.push(CalendarEvent {
                    summary: summary.clone(),
                    start: dtstart.clone(),
                    end: dtend.clone(),
                    location: location.clone(),
                });
            }
            in_vevent = false;
        } else if in_vevent {
            if let Some(val) = line.strip_prefix("SUMMARY:") {
                summary = val.to_string();
            } else if line.starts_with("DTSTART") {
                // Handle DTSTART:value and DTSTART;params:value
                if let Some(val) = line.split(':').nth(1) {
                    dtstart = val.to_string();
                }
            } else if line.starts_with("DTEND") {
                if let Some(val) = line.split(':').nth(1) {
                    dtend = val.to_string();
                }
            } else if let Some(val) = line.strip_prefix("LOCATION:") {
                location = val.to_string();
            }
        }
    }
}
