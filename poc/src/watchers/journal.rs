use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use crate::poll::sh;
use crate::state::{FullState, JournalEntry};

pub fn start_journal_watcher(state: Arc<Mutex<FullState>>, dirty: Arc<AtomicBool>) {
    std::thread::spawn(move || loop {
        let entries = read_journal_errors();
        if let Ok(mut s) = state.lock() {
            s.journal_errors = entries;
        }
        dirty.store(true, Ordering::Relaxed);
        std::thread::sleep(std::time::Duration::from_secs(30));
    });
}

fn read_journal_errors() -> Vec<JournalEntry> {
    let output =
        sh("journalctl -p 3 -n 10 --no-pager -o short 2>/dev/null").unwrap_or_default();
    let mut entries = Vec::new();
    for line in output.lines() {
        // Short format: "Mon DD HH:MM:SS hostname unit[pid]: message"
        // or:           "Mon DD HH:MM:SS hostname unit: message"
        let parts: Vec<&str> = line.splitn(5, ' ').collect();
        if parts.len() < 5 {
            continue;
        }
        let timestamp = format!("{} {} {}", parts[0], parts[1], parts[2]);
        // parts[3] is hostname, parts[4] is "unit[pid]: message" or "unit: message"
        let rest = parts[4];
        let (unit, message) = if let Some(colon_pos) = rest.find(": ") {
            let u = rest[..colon_pos]
                .trim_end_matches(|c: char| c == ']' || c.is_ascii_digit() || c == '[')
                .to_string();
            let m = rest[colon_pos + 2..].to_string();
            (u, m)
        } else {
            ("unknown".to_string(), rest.to_string())
        };
        entries.push(JournalEntry {
            unit,
            message,
            priority: 3,
            timestamp,
        });
    }
    entries.truncate(10);
    entries
}
