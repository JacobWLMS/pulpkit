//! Timer management — fires Lua interval/timeout callbacks at their scheduled times.

use std::time::{Duration, Instant};

use mlua::prelude::*;

/// A registered timer: fires a Lua callback at a fixed period or once.
pub struct ActiveTimer {
    pub id: u64,
    pub callback_key: mlua::RegistryKey,
    pub interval: Duration,
    pub next_fire: Instant,
    pub one_shot: bool,
    pub cancelled: bool,
}

/// Fire all timers that are due. Removes completed one-shots and cancelled timers.
/// Returns `true` if any fired.
pub fn fire_due_timers(timers: &mut Vec<ActiveTimer>, lua: &Lua) -> bool {
    let now = Instant::now();
    let mut any_fired = false;

    for timer in timers.iter_mut() {
        if timer.cancelled {
            continue;
        }
        if now >= timer.next_fire {
            let cb: LuaFunction = lua
                .registry_value(&timer.callback_key)
                .expect("timer: registry lookup failed");
            if let Err(e) = cb.call::<()>(()) {
                log::error!("Timer {} callback error: {e}", timer.id);
            }
            any_fired = true;
            if timer.one_shot {
                timer.cancelled = true;
            } else {
                timer.next_fire = now + timer.interval;
            }
        }
    }

    // Remove cancelled and completed one-shot timers.
    timers.retain(|t| !t.cancelled);

    any_fired
}

/// Compute the time until the next active timer fires, or a fallback default.
pub fn next_timer_timeout(timers: &[ActiveTimer], fallback: Duration) -> Duration {
    timers
        .iter()
        .filter(|t| !t.cancelled)
        .map(|t| t.next_fire.saturating_duration_since(Instant::now()))
        .min()
        .unwrap_or(fallback)
        .max(Duration::from_millis(1))
}
