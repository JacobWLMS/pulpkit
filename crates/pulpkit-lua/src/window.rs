use mlua::prelude::*;
use mlua::RegistryKey;
use std::cell::RefCell;
use std::rc::Rc;

/// Description of a window collected during shell.lua execution.
pub struct WindowDef {
    pub name: String,
    pub monitor: MonitorTarget,
    pub anchor: String,
    pub exclusive: bool,
    pub height: Option<u32>,
    pub width: Option<u32>,
    pub namespace: String,
    /// Registry key referencing the Lua widget-builder function.
    pub widget_fn: RegistryKey,
}

/// Which monitor(s) a window should appear on.
pub enum MonitorTarget {
    All,
    Focused,
    Named(String),
}

/// Registry of window definitions collected during shell.lua execution.
pub type WindowRegistry = Rc<RefCell<Vec<WindowDef>>>;

/// Register the global `window(name, opts, widget_fn)` function.
pub fn register_window_fn(lua: &Lua, registry: WindowRegistry) -> LuaResult<()> {
    let window_fn =
        lua.create_function(move |lua, (name, opts, func): (String, LuaTable, LuaFunction)| {
            let monitor_str: Option<String> = opts.get("monitor")?;
            let monitor = match monitor_str.as_deref() {
                Some("all") => MonitorTarget::All,
                Some("focused") | None => MonitorTarget::Focused,
                Some(name) => MonitorTarget::Named(name.to_string()),
            };

            let anchor: String = opts.get::<Option<String>>("anchor")?.unwrap_or_default();
            let exclusive: bool = opts.get::<Option<bool>>("exclusive")?.unwrap_or(false);
            let height: Option<u32> = opts.get("height")?;
            let width: Option<u32> = opts.get("width")?;
            let namespace: String = opts
                .get::<Option<String>>("namespace")?
                .unwrap_or_else(|| name.clone());

            let widget_fn = lua.create_registry_value(func)?;

            registry.borrow_mut().push(WindowDef {
                name,
                monitor,
                anchor,
                exclusive,
                height,
                width,
                namespace,
                widget_fn,
            });

            Ok(())
        })?;

    lua.globals().set("window", window_fn)?;
    Ok(())
}
