use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use zbus::blocking::{Connection, MessageIterator};
use zbus::{MatchRule, MessageType};

use crate::poll::sh;
use crate::state::{DriveInfo, FullState};

/// Parse lsblk JSON output to find removable, mounted drives.
fn read_drives() -> Vec<DriveInfo> {
    let output = sh("lsblk -J -b -o NAME,MOUNTPOINT,SIZE,TYPE,RM,FSTYPE 2>/dev/null")
        .unwrap_or_default();
    let parsed: serde_json::Value = match serde_json::from_str(&output) {
        Ok(v) => v,
        Err(_) => return vec![],
    };

    let mut drives = Vec::new();
    collect_drives(&parsed["blockdevices"], &mut drives);
    drives
}

/// Recursively walk blockdevices (and their children) to find removable mounted partitions.
fn collect_drives(devices: &serde_json::Value, out: &mut Vec<DriveInfo>) {
    let Some(arr) = devices.as_array() else {
        return;
    };
    for dev in arr {
        let rm = dev["rm"]
            .as_bool()
            .or_else(|| dev["rm"].as_str().map(|s| s == "1" || s == "true"))
            .unwrap_or(false);
        let dev_type = dev["type"].as_str().unwrap_or("");
        let mountpoint = dev["mountpoint"].as_str().unwrap_or("");
        let name = dev["name"].as_str().unwrap_or("");
        let size = dev["size"]
            .as_u64()
            .or_else(|| dev["size"].as_str().and_then(|s| s.parse().ok()))
            .unwrap_or(0);

        if rm && dev_type == "part" && !mountpoint.is_empty() {
            out.push(DriveInfo {
                name: name.to_string(),
                mount_point: mountpoint.to_string(),
                size_bytes: size,
                device: format!("/dev/{name}"),
            });
        }

        // Recurse into children (partitions nested under parent disk)
        if let Some(children) = dev.get("children") {
            // Children inherit removable flag from parent
            if rm {
                collect_children_rm(children, out);
            }
        }
    }
}

/// Collect mounted partitions from children array, knowing the parent is removable.
fn collect_children_rm(children: &serde_json::Value, out: &mut Vec<DriveInfo>) {
    let Some(arr) = children.as_array() else {
        return;
    };
    for dev in arr {
        let dev_type = dev["type"].as_str().unwrap_or("");
        let mountpoint = dev["mountpoint"].as_str().unwrap_or("");
        let name = dev["name"].as_str().unwrap_or("");
        let size = dev["size"]
            .as_u64()
            .or_else(|| dev["size"].as_str().and_then(|s| s.parse().ok()))
            .unwrap_or(0);

        if dev_type == "part" && !mountpoint.is_empty() {
            out.push(DriveInfo {
                name: name.to_string(),
                mount_point: mountpoint.to_string(),
                size_bytes: size,
                device: format!("/dev/{name}"),
            });
        }

        // Recurse further if nested
        if let Some(grandchildren) = dev.get("children") {
            collect_children_rm(grandchildren, out);
        }
    }
}

fn update_state(state: &Arc<Mutex<FullState>>, dirty: &Arc<AtomicBool>) {
    let drives = read_drives();
    if let Ok(mut s) = state.lock() {
        s.drives = drives;
    }
    dirty.store(true, Ordering::Relaxed);
}

pub fn start_udisks_watcher(state: Arc<Mutex<FullState>>, dirty: Arc<AtomicBool>) {
    std::thread::spawn(move || {
        let conn = match Connection::system() {
            Ok(c) => c,
            Err(e) => {
                log::warn!("[udisks] failed to connect to system bus: {e}");
                return;
            }
        };

        // Initial read
        update_state(&state, &dirty);

        // Subscribe to all signals from UDisks2 (InterfacesAdded/Removed, PropertiesChanged)
        let rule = match MatchRule::builder()
            .msg_type(MessageType::Signal)
            .sender("org.freedesktop.UDisks2")
        {
            Ok(b) => b.build(),
            Err(e) => {
                log::warn!("[udisks] failed to build match rule: {e}");
                return;
            }
        };

        let mut iter = match MessageIterator::for_match_rule(rule, &conn, Some(16)) {
            Ok(it) => it,
            Err(e) => {
                log::warn!("[udisks] failed to create message iterator: {e}");
                return;
            }
        };

        // On any signal from UDisks2, re-read drives via lsblk
        while let Some(Ok(_msg)) = iter.next() {
            update_state(&state, &dirty);
        }
    });
}
