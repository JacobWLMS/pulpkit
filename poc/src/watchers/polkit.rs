use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use zbus::blocking::Connection;
use zbus::{MatchRule, MessageType};

use crate::state::FullState;

const POLKIT_DEST: &str = "org.freedesktop.PolicyKit1";
const POLKIT_PATH: &str = "/org/freedesktop/PolicyKit1/Authority";


pub fn start_polkit_watcher(state: Arc<Mutex<FullState>>, dirty: Arc<AtomicBool>) {
    std::thread::spawn(move || {
        let conn = match Connection::system() {
            Ok(c) => c,
            Err(e) => {
                log::warn!("[polkit] failed to connect to system bus: {e}");
                return;
            }
        };

        // Subscribe to signals from PolicyKit1 Authority
        let rule = match MatchRule::builder()
            .msg_type(MessageType::Signal)
            .sender(POLKIT_DEST)
        {
            Ok(b) => match b.path(POLKIT_PATH) {
                Ok(b2) => b2.build(),
                Err(_) => return,
            },
            Err(_) => return,
        };

        let mut iter = match zbus::blocking::MessageIterator::for_match_rule(rule, &conn, Some(16))
        {
            Ok(it) => it,
            Err(e) => {
                log::warn!("[polkit] failed to create message iterator: {e}");
                return;
            }
        };

        let mut last_signal: Option<Instant> = None;
        let timeout = std::time::Duration::from_secs(30);

        loop {
            // Check if we should clear the pending state
            if let Some(ts) = last_signal {
                if ts.elapsed() >= timeout {
                    if let Ok(mut s) = state.lock() {
                        if s.polkit_pending {
                            s.polkit_pending = false;
                            s.polkit_message.clear();
                            dirty.store(true, Ordering::Relaxed);
                        }
                    }
                    last_signal = None;
                }
            }

            // Non-blocking check: use a short timeout via next()
            // MessageIterator::next() blocks, so we use a polling approach
            // by checking with a timeout thread. For simplicity, just block
            // and handle the signal.
            match iter.next() {
                Some(Ok(msg)) => {
                    let member = msg
                        .header()
                        .ok()
                        .and_then(|h| h.member().ok().flatten().map(|m| m.as_str().to_string()))
                        .unwrap_or_default();

                    if member == "Changed" {
                        last_signal = Some(Instant::now());
                        if let Ok(mut s) = state.lock() {
                            s.polkit_pending = true;
                            s.polkit_message = "Authentication requested".to_string();
                        }
                        dirty.store(true, Ordering::Relaxed);
                    }
                }
                Some(Err(_)) => continue,
                None => break,
            }
        }
    });
}
