use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use crate::state::FullState;

pub fn start_net_speed_watcher(state: Arc<Mutex<FullState>>, dirty: Arc<AtomicBool>) {
    std::thread::spawn(move || {
        let mut prev_rx: u64 = 0;
        let mut prev_tx: u64 = 0;
        let mut first = true;

        loop {
            let (rx, tx) = read_net_bytes();
            if !first {
                let rx_delta = rx.saturating_sub(prev_rx);
                let tx_delta = tx.saturating_sub(prev_tx);
                // Delta per second (we sleep 2s, so divide by 2)
                if let Ok(mut s) = state.lock() {
                    s.net_rx_bytes_sec = rx_delta / 2;
                    s.net_tx_bytes_sec = tx_delta / 2;
                }
                dirty.store(true, Ordering::Relaxed);
            }
            first = false;
            prev_rx = rx;
            prev_tx = tx;
            std::thread::sleep(std::time::Duration::from_secs(2));
        }
    });
}

/// Read total RX and TX bytes from /proc/net/dev, summing all non-loopback interfaces.
fn read_net_bytes() -> (u64, u64) {
    let content = std::fs::read_to_string("/proc/net/dev").unwrap_or_default();
    let mut total_rx = 0u64;
    let mut total_tx = 0u64;
    for line in content.lines().skip(2) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 10 { continue; }
        let iface = parts[0].trim_end_matches(':');
        if iface == "lo" { continue; } // skip loopback
        if let Ok(rx) = parts[1].parse::<u64>() { total_rx += rx; }
        if let Ok(tx) = parts[9].parse::<u64>() { total_tx += tx; }
    }
    (total_rx, total_tx)
}
