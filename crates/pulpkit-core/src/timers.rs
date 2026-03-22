//! Timer management — fires Lua interval callbacks at their scheduled times.

use std::time::{Duration, Instant};

use mlua::prelude::*;

/// A registered interval: fires a Lua callback at a fixed period.
pub struct ActiveInterval {
    pub callback_key: mlua::RegistryKey,
    pub interval: Duration,
    pub next_fire: Instant,
}

/// Fire all intervals that are due. Returns `true` if any fired.
pub fn fire_due_intervals(intervals: &mut [ActiveInterval], lua: &Lua) -> bool {
    let now = Instant::now();
    let mut any_fired = false;

    for interval in intervals.iter_mut() {
        if now >= interval.next_fire {
            let cb: LuaFunction = lua
                .registry_value(&interval.callback_key)
                .expect("interval: registry lookup failed");
            if let Err(e) = cb.call::<()>(()) {
                log::error!("Interval callback error: {e}");
            }
            interval.next_fire = now + interval.interval;
            any_fired = true;
        }
    }

    any_fired
}

/// Compute the time until the next interval fires, or a fallback default.
pub fn next_interval_timeout(intervals: &[ActiveInterval], fallback: Duration) -> Duration {
    intervals
        .iter()
        .map(|i| i.next_fire.saturating_duration_since(Instant::now()))
        .min()
        .unwrap_or(fallback)
        .max(Duration::from_millis(1)) // prevent busy-loop
}
