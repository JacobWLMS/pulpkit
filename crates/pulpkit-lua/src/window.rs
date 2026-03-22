use mlua::prelude::*;
use mlua::RegistryKey;
use std::cell::RefCell;
use std::rc::Rc;

use crate::signals::LuaSignal;
use pulpkit_reactive::{DynValue, Signal};

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

// ---------------------------------------------------------------------------
// Popup definitions
// ---------------------------------------------------------------------------

/// Description of a popup collected during shell.lua execution.
pub struct PopupDef {
    /// Unique name for this popup.
    pub name: String,
    /// Name of the parent window (used for positioning context).
    pub parent: String,
    /// Anchor string: "top left", "top right", "bottom left", "bottom right".
    pub anchor: String,
    /// (x, y) offset from the anchor point.
    pub offset: (i32, i32),
    /// Whether clicking outside the popup dismisses it.
    pub dismiss_on_outside: bool,
    /// The reactive signal controlling visibility.
    pub visible_signal: Option<Signal<DynValue>>,
    /// Registry key referencing the Lua widget-builder function.
    pub widget_fn_key: RegistryKey,
    /// Explicit width, if provided.
    pub width: Option<u32>,
    /// Explicit height, if provided.
    pub height: Option<u32>,
}

/// Registry of popup definitions collected during shell.lua execution.
pub type PopupRegistry = Rc<RefCell<Vec<PopupDef>>>;

/// Register the global `popup(name, opts, widget_fn)` function.
pub fn register_popup_fn(lua: &Lua, registry: PopupRegistry) -> LuaResult<()> {
    let popup_fn =
        lua.create_function(move |lua, (name, opts, func): (String, LuaTable, LuaFunction)| {
            let parent: String = opts
                .get::<Option<String>>("parent")?
                .unwrap_or_default();
            let anchor: String = opts
                .get::<Option<String>>("anchor")?
                .unwrap_or_else(|| "top left".to_string());

            // Parse offset table: { x = ..., y = ... }
            let offset = if let Ok(offset_table) = opts.get::<LuaTable>("offset") {
                let x: i32 = offset_table.get::<Option<i32>>("x")?.unwrap_or(0);
                let y: i32 = offset_table.get::<Option<i32>>("y")?.unwrap_or(0);
                (x, y)
            } else {
                (0, 0)
            };

            let dismiss_on_outside: bool = opts
                .get::<Option<bool>>("dismiss_on_outside")?
                .unwrap_or(false);

            // Extract the visible signal (LuaSignal userdata).
            let visible_signal = if let Ok(ud) = opts.get::<LuaAnyUserData>("visible") {
                let lua_sig = ud.borrow::<LuaSignal>()?;
                Some(lua_sig.0.clone())
            } else {
                None
            };

            let width: Option<u32> = opts.get("width")?;
            let height: Option<u32> = opts.get("height")?;

            let widget_fn_key = lua.create_registry_value(func)?;

            registry.borrow_mut().push(PopupDef {
                name,
                parent,
                anchor,
                offset,
                dismiss_on_outside,
                visible_signal,
                widget_fn_key,
                width,
                height,
            });

            Ok(())
        })?;

    lua.globals().set("popup", popup_fn)?;
    Ok(())
}
