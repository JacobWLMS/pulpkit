//! Timer support — `set_interval(fn, ms)` for periodic Lua callbacks.
//!
//! Since calloop timer callbacks require `Send` (and `LuaFunction` is not),
//! we use a registry approach: Lua calls `set_interval()` to register
//! interval definitions, and the runtime processes them manually in the
//! event loop using `Instant`-based tracking.

use mlua::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

/// A single interval definition collected from Lua.
pub struct IntervalDef {
    /// Registry key for the Lua callback function.
    pub callback_key: mlua::RegistryKey,
    /// Interval period in milliseconds.
    pub interval_ms: u64,
}

/// Shared registry that collects interval definitions from Lua's `set_interval()`.
pub type IntervalRegistry = Rc<RefCell<Vec<IntervalDef>>>;

/// Register the `set_interval(fn, ms)` global function in Lua.
///
/// The function stores interval definitions in the shared registry.
/// The runtime processes these after shell.lua loads, setting up
/// manual timer tracking in the event loop.
pub fn register_interval_fn(lua: &Lua, registry: IntervalRegistry) -> LuaResult<()> {
    let set_interval_fn = lua.create_function(move |lua, (callback, ms): (LuaFunction, u64)| {
        let key = lua.create_registry_value(callback)?;
        registry.borrow_mut().push(IntervalDef {
            callback_key: key,
            interval_ms: ms,
        });
        Ok(())
    })?;
    lua.globals().set("set_interval", set_interval_fn)?;
    Ok(())
}
