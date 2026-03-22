//! Timer support — `set_interval`, `set_timeout`, `clear_interval`, `clear_timeout`.
//!
//! Since calloop timer callbacks require `Send` (and `LuaFunction` is not),
//! we use a registry approach: Lua calls these functions to register timer
//! definitions, and the runtime processes them manually in the event loop.

use mlua::prelude::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// A single timer definition collected from Lua.
pub struct TimerDef {
    /// Unique timer ID (returned to Lua for cancellation).
    pub id: u64,
    /// Registry key for the Lua callback function.
    pub callback_key: mlua::RegistryKey,
    /// Period in milliseconds.
    pub interval_ms: u64,
    /// If true, fires once and then auto-removes.
    pub one_shot: bool,
}

/// Shared registry that collects timer definitions from Lua.
pub type TimerRegistry = Rc<RefCell<Vec<TimerDef>>>;

/// IDs of timers cancelled from Lua before the runtime processes them.
pub type CancelledTimers = Rc<RefCell<Vec<u64>>>;

/// Thread-local auto-incrementing timer ID counter.
fn next_timer_id() -> u64 {
    thread_local! {
        static COUNTER: Cell<u64> = const { Cell::new(1) };
    }
    COUNTER.with(|c| {
        let id = c.get();
        c.set(id + 1);
        id
    })
}

/// Register `set_interval(fn, ms)`, `set_timeout(fn, ms)`,
/// `clear_interval(id)`, and `clear_timeout(id)` as Lua globals.
pub fn register_timer_api(
    lua: &Lua,
    registry: TimerRegistry,
    cancelled: CancelledTimers,
) -> LuaResult<()> {
    // set_interval(fn, ms) -> id
    {
        let reg = registry.clone();
        let f = lua.create_function(move |lua, (callback, ms): (LuaFunction, u64)| {
            let id = next_timer_id();
            let key = lua.create_registry_value(callback)?;
            reg.borrow_mut().push(TimerDef {
                id,
                callback_key: key,
                interval_ms: ms,
                one_shot: false,
            });
            Ok(id)
        })?;
        lua.globals().set("set_interval", f)?;
    }

    // set_timeout(fn, ms) -> id
    {
        let reg = registry.clone();
        let f = lua.create_function(move |lua, (callback, ms): (LuaFunction, u64)| {
            let id = next_timer_id();
            let key = lua.create_registry_value(callback)?;
            reg.borrow_mut().push(TimerDef {
                id,
                callback_key: key,
                interval_ms: ms,
                one_shot: true,
            });
            Ok(id)
        })?;
        lua.globals().set("set_timeout", f)?;
    }

    // clear_interval(id)
    {
        let canc = cancelled.clone();
        let f = lua.create_function(move |_lua, id: u64| {
            canc.borrow_mut().push(id);
            Ok(())
        })?;
        lua.globals().set("clear_interval", f)?;
    }

    // clear_timeout(id) — same as clear_interval
    {
        let canc = cancelled.clone();
        let f = lua.create_function(move |_lua, id: u64| {
            canc.borrow_mut().push(id);
            Ok(())
        })?;
        lua.globals().set("clear_timeout", f)?;
    }

    Ok(())
}
