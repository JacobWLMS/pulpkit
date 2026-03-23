use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use crate::state::{FullState, RecentFile};

pub fn start_recent_files_watcher(state: Arc<Mutex<FullState>>, dirty: Arc<AtomicBool>) {
    std::thread::spawn(move || loop {
        let files = read_recent_files();
        if let Ok(mut s) = state.lock() {
            s.recent_files = files;
        }
        dirty.store(true, Ordering::Relaxed);
        std::thread::sleep(std::time::Duration::from_secs(30));
    });
}

fn read_recent_files() -> Vec<RecentFile> {
    let home = std::env::var("HOME").unwrap_or_default();
    let path = format!("{home}/.local/share/recently-used.xbel");
    let Ok(content) = std::fs::read_to_string(&path) else {
        return Vec::new();
    };

    let mut files = Vec::new();

    // Simple string parsing for <bookmark href="..." modified="..." ...>
    // and nested <mime:mime-type type="..."/>
    let mut i = 0;
    let bytes = content.as_bytes();
    while i < bytes.len() {
        // Find <bookmark
        if let Some(pos) = content[i..].find("<bookmark ") {
            let start = i + pos;
            // Find the closing </bookmark> to scope mime-type search
            let block_end = content[start..]
                .find("</bookmark>")
                .map(|p| start + p + 11)
                .unwrap_or(content.len());
            let tag_region = &content[start..block_end];

            let href = extract_attr(tag_region, "href=\"");
            let modified = extract_attr(tag_region, "modified=\"");
            let mime = extract_attr(tag_region, "mime:mime-type type=\"")
                .or_else(|| extract_attr(tag_region, "type=\""));

            if let Some(uri) = href {
                let name = uri
                    .rsplit('/')
                    .next()
                    .unwrap_or(&uri)
                    .to_string();
                // Decode percent-encoded name
                let name = percent_decode(&name);
                let timestamp = parse_iso_timestamp(&modified.unwrap_or_default());
                files.push(RecentFile {
                    name,
                    uri,
                    mime_type: mime.unwrap_or_default(),
                    timestamp,
                });
            }
            i = block_end;
        } else {
            break;
        }
    }

    // Sort by timestamp descending (most recent first)
    files.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    files.truncate(10);
    files
}

fn extract_attr(s: &str, prefix: &str) -> Option<String> {
    let start = s.find(prefix)?;
    let after = &s[start + prefix.len()..];
    let end = after.find('"')?;
    Some(after[..end].to_string())
}

fn percent_decode(s: &str) -> String {
    let mut result = Vec::new();
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(byte) = u8::from_str_radix(
                std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or(""),
                16,
            ) {
                result.push(byte);
                i += 3;
                continue;
            }
        }
        result.push(bytes[i]);
        i += 1;
    }
    String::from_utf8(result).unwrap_or_else(|_| s.to_string())
}

fn parse_iso_timestamp(s: &str) -> u64 {
    // Parse ISO 8601 timestamps like "2024-01-15T10:30:00Z"
    // Simple approach: extract digits and compute approximate epoch
    let clean: String = s
        .chars()
        .filter(|c| c.is_ascii_digit())
        .collect();
    if clean.len() >= 14 {
        // YYYYMMDDHHmmss — convert to a sortable number
        clean[..14].parse::<u64>().unwrap_or(0)
    } else {
        0
    }
}
