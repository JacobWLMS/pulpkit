use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use crate::poll::{poll_net_details, poll_wifi};
use crate::state::FullState;

fn update_network_state(state: &Arc<Mutex<FullState>>, dirty: &Arc<AtomicBool>) {
    let wifi = poll_wifi();
    let (net_signal, net_ip) = poll_net_details();
    if let Ok(mut s) = state.lock() {
        s.wifi = wifi;
        s.net_signal = net_signal;
        s.net_ip = net_ip;
    }
    dirty.store(true, Ordering::Relaxed);
}

pub fn start_network_watcher(state: Arc<Mutex<FullState>>, dirty: Arc<AtomicBool>) {
    std::thread::spawn(move || {
        // Initial read so we have network state before any signals arrive
        update_network_state(&state, &dirty);

        let conn = match zbus::blocking::Connection::system() {
            Ok(c) => c,
            Err(e) => {
                eprintln!("[pulpkit] network watcher: failed to connect to system bus: {e}");
                return;
            }
        };

        let rule = match zbus::MatchRule::builder()
            .msg_type(zbus::MessageType::Signal)
            .sender("org.freedesktop.NetworkManager")
        {
            Ok(b) => b.build(),
            Err(e) => {
                eprintln!("[pulpkit] network watcher: failed to build match rule: {e}");
                return;
            }
        };

        let iter = match zbus::blocking::MessageIterator::for_match_rule(rule, &conn, None) {
            Ok(it) => it,
            Err(e) => {
                eprintln!("[pulpkit] network watcher: failed to set up message iterator: {e}");
                return;
            }
        };

        for _msg in iter {
            update_network_state(&state, &dirty);
        }
    });
}
