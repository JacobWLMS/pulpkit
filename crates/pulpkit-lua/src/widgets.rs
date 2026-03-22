use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use mlua::prelude::*;
use pulpkit_layout::tree::{ButtonHandlers, Direction, Node, SliderState, ToggleState};
use pulpkit_layout::style::StyleProps;
use pulpkit_layout::Theme;
use pulpkit_render::Color;

use crate::signals::LuaSignal;

/// Wrapper to make `Node` work as Lua UserData.
#[derive(Clone)]
pub struct LuaNode(pub Node);

impl LuaUserData for LuaNode {}

/// Extract a `Vec<Node>` from a Lua table of `LuaNode` userdata values.
fn table_to_nodes(table: &LuaTable) -> LuaResult<Vec<Node>> {
    let mut nodes = Vec::new();
    for value in table.sequence_values::<LuaAnyUserData>() {
        let ud = value?;
        let node = ud.borrow::<LuaNode>()?;
        nodes.push(node.0.clone());
    }
    Ok(nodes)
}

/// Register `row`, `col`, `box`, `text`, and `spacer` as Lua globals.
pub fn register_widgets(lua: &Lua, theme: Arc<Theme>) -> LuaResult<()> {
    // row(style_string, children_table) -> LuaNode
    let t = theme.clone();
    let row_fn = lua.create_function(move |_lua, (style_str, children): (String, LuaTable)| {
        let style = StyleProps::parse(&style_str, &t);
        let nodes = table_to_nodes(&children)?;
        Ok(LuaNode(Node::Container {
            style,
            direction: Direction::Row,
            children: nodes,
        }))
    })?;
    lua.globals().set("row", row_fn)?;

    // col(style_string, children_table) -> LuaNode
    let t = theme.clone();
    let col_fn = lua.create_function(move |_lua, (style_str, children): (String, LuaTable)| {
        let style = StyleProps::parse(&style_str, &t);
        let nodes = table_to_nodes(&children)?;
        Ok(LuaNode(Node::Container {
            style,
            direction: Direction::Column,
            children: nodes,
        }))
    })?;
    lua.globals().set("col", col_fn)?;

    // box(style_string, children_table) -> LuaNode  (alias for col)
    let t = theme.clone();
    let box_fn = lua.create_function(move |_lua, (style_str, children): (String, LuaTable)| {
        let style = StyleProps::parse(&style_str, &t);
        let nodes = table_to_nodes(&children)?;
        Ok(LuaNode(Node::Container {
            style,
            direction: Direction::Column,
            children: nodes,
        }))
    })?;
    // "box" is a Lua keyword-safe name (not reserved), register it directly.
    lua.globals().set("box", box_fn)?;

    // text(style_string, content_string) -> LuaNode
    let t = theme.clone();
    let text_fn = lua.create_function(move |_lua, (style_str, content): (String, String)| {
        let style = StyleProps::parse(&style_str, &t);
        Ok(LuaNode(Node::Text { style, content }))
    })?;
    lua.globals().set("text", text_fn)?;

    // spacer() -> LuaNode
    let spacer_fn = lua.create_function(|_lua, ()| Ok(LuaNode(Node::Spacer)))?;
    lua.globals().set("spacer", spacer_fn)?;

    // button(style_string, opts_table, children_table) -> LuaNode
    //
    // opts may contain on_click, on_scroll_up, on_scroll_down (Lua functions).
    // Children is optional — if omitted, the button has no child nodes.
    let t = theme.clone();
    let button_fn = lua.create_function(move |lua, (style_str, opts, children): (String, LuaTable, Option<LuaTable>)| {
        let style = StyleProps::parse(&style_str, &t);

        let nodes = match children {
            Some(tbl) => table_to_nodes(&tbl)?,
            None => Vec::new(),
        };

        let handlers = lua_table_to_handlers(lua, &opts)?;

        Ok(LuaNode(Node::Button {
            style,
            children: nodes,
            handlers,
        }))
    })?;
    lua.globals().set("button", button_fn)?;

    // slider(style_string, opts_table) -> LuaNode
    //
    // opts:
    //   value     — LuaSignal (current numeric value)
    //   on_change — function(v) called when the value changes from interaction
    //   min       — number (default 0)
    //   max       — number (default 100)
    //
    // Style tokens:
    //   accent-<color>  — parsed manually for the filled track color
    //   (all other tokens handled normally by StyleProps::parse)
    let t = theme.clone();
    let slider_fn = lua.create_function(move |lua, (style_str, opts): (String, LuaTable)| {
        // Parse accent-* token manually before passing to StyleProps::parse.
        let mut accent_color = None;
        let mut filtered_tokens = Vec::new();
        for token in style_str.split_whitespace() {
            if let Some(color_name) = token.strip_prefix("accent-") {
                if let Some(&c) = t.colors.get(color_name) {
                    accent_color = Some(c);
                }
                // Don't pass accent-* to StyleProps::parse (it would be unknown)
            } else {
                filtered_tokens.push(token);
            }
        }
        let style = StyleProps::parse(&filtered_tokens.join(" "), &t);

        // Read the value signal's current value.
        let value_signal: LuaAnyUserData = opts.get("value")?;
        let sig = value_signal.borrow::<LuaSignal>()?;
        let current_val = match sig.0.get() {
            crate::signals::DynValue::Int(i) => i as f64,
            crate::signals::DynValue::Float(f) => f,
            _ => 0.0,
        };

        let min: f64 = opts.get::<Option<f64>>("min")?.unwrap_or(0.0);
        let max: f64 = opts.get::<Option<f64>>("max")?.unwrap_or(100.0);

        // Wrap on_change Lua function.
        let on_change = wrap_lua_callback_f64(lua, opts.get::<Option<LuaFunction>>("on_change")?)?;

        let state = SliderState {
            value: Rc::new(RefCell::new(current_val)),
            min,
            max,
            on_change,
            accent_color,
        };

        Ok(LuaNode(Node::Slider { style, state }))
    })?;
    lua.globals().set("slider", slider_fn)?;

    // toggle(style_string, opts_table) -> LuaNode
    //
    // opts:
    //   checked   — LuaSignal (current boolean value)
    //   on_change — function(v) called when the toggle is flipped
    //
    // Style tokens:
    //   accent-<color>  — parsed manually for the track color when checked
    //   (all other tokens handled normally by StyleProps::parse)
    let t = theme.clone();
    let toggle_fn = lua.create_function(move |lua, (style_str, opts): (String, LuaTable)| {
        // Parse accent-* token manually before passing to StyleProps::parse.
        let mut accent_color = None;
        let mut filtered_tokens = Vec::new();
        for token in style_str.split_whitespace() {
            if let Some(color_name) = token.strip_prefix("accent-") {
                if let Some(&c) = t.colors.get(color_name) {
                    accent_color = Some(c);
                }
            } else {
                filtered_tokens.push(token);
            }
        }
        let style = StyleProps::parse(&filtered_tokens.join(" "), &t);

        // Read the checked signal's current value.
        let checked_signal: LuaAnyUserData = opts.get("checked")?;
        let sig = checked_signal.borrow::<LuaSignal>()?;
        let current_val = matches!(sig.0.get(), crate::signals::DynValue::Bool(true));

        // Wrap on_change Lua function.
        let on_change = wrap_lua_callback_bool(lua, opts.get::<Option<LuaFunction>>("on_change")?)?;

        let state = ToggleState {
            checked: Rc::new(RefCell::new(current_val)),
            on_change,
            accent_color,
        };

        Ok(LuaNode(Node::Toggle { style, state }))
    })?;
    lua.globals().set("toggle", toggle_fn)?;

    Ok(())
}

/// Extract event-handler Lua functions from an options table and wrap them
/// as `ButtonHandlers` with `Rc`-wrapped closures.
fn lua_table_to_handlers(lua: &Lua, opts: &LuaTable) -> LuaResult<ButtonHandlers> {
    let on_click = wrap_lua_callback(lua, opts.get::<Option<LuaFunction>>("on_click")?)?;
    let on_scroll_up = wrap_lua_callback(lua, opts.get::<Option<LuaFunction>>("on_scroll_up")?)?;
    let on_scroll_down = wrap_lua_callback(lua, opts.get::<Option<LuaFunction>>("on_scroll_down")?)?;

    Ok(ButtonHandlers {
        on_click,
        on_scroll_up,
        on_scroll_down,
    })
}

/// Store a Lua function in the registry and return an `Rc<dyn Fn()>` that
/// calls it. Returns `None` if the input is `None`.
fn wrap_lua_callback(lua: &Lua, func: Option<LuaFunction>) -> LuaResult<Option<Rc<dyn Fn()>>> {
    match func {
        None => Ok(None),
        Some(f) => {
            let key = lua.create_registry_value(f)?;
            let lua_clone = lua.clone();
            Ok(Some(Rc::new(move || {
                if let Ok(func) = lua_clone.registry_value::<LuaFunction>(&key) {
                    if let Err(e) = func.call::<()>(()) {
                        log::error!("Button handler error: {e}");
                    }
                }
            })))
        }
    }
}

/// Store a Lua function in the registry and return an `Rc<dyn Fn(f64)>` that
/// calls it with an `f64` argument. Returns `None` if the input is `None`.
fn wrap_lua_callback_f64(lua: &Lua, func: Option<LuaFunction>) -> LuaResult<Option<Rc<dyn Fn(f64)>>> {
    match func {
        None => Ok(None),
        Some(f) => {
            let key = lua.create_registry_value(f)?;
            let lua_clone = lua.clone();
            Ok(Some(Rc::new(move |v: f64| {
                if let Ok(func) = lua_clone.registry_value::<LuaFunction>(&key) {
                    if let Err(e) = func.call::<()>(v) {
                        log::error!("Slider on_change handler error: {e}");
                    }
                }
            })))
        }
    }
}

/// Store a Lua function in the registry and return an `Rc<dyn Fn(bool)>` that
/// calls it with a `bool` argument. Returns `None` if the input is `None`.
fn wrap_lua_callback_bool(lua: &Lua, func: Option<LuaFunction>) -> LuaResult<Option<Rc<dyn Fn(bool)>>> {
    match func {
        None => Ok(None),
        Some(f) => {
            let key = lua.create_registry_value(f)?;
            let lua_clone = lua.clone();
            Ok(Some(Rc::new(move |v: bool| {
                if let Ok(func) = lua_clone.registry_value::<LuaFunction>(&key) {
                    if let Err(e) = func.call::<()>(v) {
                        log::error!("Toggle on_change handler error: {e}");
                    }
                }
            })))
        }
    }
}
