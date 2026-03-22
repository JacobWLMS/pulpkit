use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use mlua::prelude::*;
use pulpkit_layout::style::StyleProps;
use pulpkit_layout::tree::{Direction, EventHandlers, InteractiveKind, Node, Prop};
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

/// Convert a Lua value (string or function) into a `Prop<StyleProps>`.
///
/// - String: parsed once as static style.
/// - Function: called each frame; result is parsed (with caching to avoid
///   re-parsing when the returned string hasn't changed).
/// - Other: returns `Prop::Static(StyleProps::default())`.
fn value_to_style_prop(lua: &Lua, val: LuaValue, theme: &Arc<Theme>) -> LuaResult<Prop<StyleProps>> {
    match val {
        LuaValue::String(s) => {
            let parsed = StyleProps::parse(&s.to_string_lossy(), theme);
            Ok(Prop::Static(parsed))
        }
        LuaValue::Function(f) => {
            let key = lua.create_registry_value(f)?;
            let lua_clone = lua.clone();
            let theme = theme.clone();
            let cached_str = RefCell::new(String::new());
            let cached_props = RefCell::new(StyleProps::default());
            Ok(Prop::Reactive(Rc::new(move || {
                let cb: LuaFunction = lua_clone.registry_value(&key).unwrap();
                let new_str: String = cb.call(()).unwrap_or_default();
                if new_str != *cached_str.borrow() {
                    *cached_str.borrow_mut() = new_str.clone();
                    *cached_props.borrow_mut() = StyleProps::parse(&new_str, &theme);
                }
                cached_props.borrow().clone()
            })))
        }
        _ => Ok(Prop::Static(StyleProps::default())),
    }
}

/// Convert a Lua value (string, number, or function) into a `Prop<String>`.
fn value_to_string_prop(lua: &Lua, val: LuaValue) -> LuaResult<Prop<String>> {
    match val {
        LuaValue::String(s) => Ok(Prop::Static(s.to_string_lossy().to_string())),
        LuaValue::Function(f) => {
            let key = lua.create_registry_value(f)?;
            let lua_clone = lua.clone();
            Ok(Prop::Reactive(Rc::new(move || {
                let cb: LuaFunction = lua_clone.registry_value(&key).unwrap();
                cb.call::<String>(()).unwrap_or_default()
            })))
        }
        LuaValue::Integer(i) => Ok(Prop::Static(i.to_string())),
        LuaValue::Number(n) => Ok(Prop::Static(n.to_string())),
        _ => Ok(Prop::Static(String::new())),
    }
}

/// Register `row`, `col`, `box`, `text`, `spacer`, `button`, `slider`, and
/// `toggle` as Lua globals.
pub fn register_widgets(lua: &Lua, theme: Arc<Theme>) -> LuaResult<()> {
    // row(style, children) -> LuaNode
    {
        let t = theme.clone();
        let row_fn = lua.create_function(move |lua, (style_val, children): (LuaValue, LuaTable)| {
            let style = value_to_style_prop(lua, style_val, &t)?;
            let nodes = table_to_nodes(&children)?;
            Ok(LuaNode(Node::Container {
                style,
                direction: Direction::Row,
                children: nodes,
            }))
        })?;
        lua.globals().set("row", row_fn)?;
    }

    // col(style, children) -> LuaNode
    {
        let t = theme.clone();
        let col_fn = lua.create_function(move |lua, (style_val, children): (LuaValue, LuaTable)| {
            let style = value_to_style_prop(lua, style_val, &t)?;
            let nodes = table_to_nodes(&children)?;
            Ok(LuaNode(Node::Container {
                style,
                direction: Direction::Column,
                children: nodes,
            }))
        })?;
        lua.globals().set("col", col_fn)?;
    }

    // box(style, children) -> LuaNode  (alias for col)
    {
        let t = theme.clone();
        let box_fn = lua.create_function(move |lua, (style_val, children): (LuaValue, LuaTable)| {
            let style = value_to_style_prop(lua, style_val, &t)?;
            let nodes = table_to_nodes(&children)?;
            Ok(LuaNode(Node::Container {
                style,
                direction: Direction::Column,
                children: nodes,
            }))
        })?;
        lua.globals().set("box", box_fn)?;
    }

    // text(style, content) -> LuaNode
    // Both style and content accept static values or functions for reactivity.
    {
        let t = theme.clone();
        let text_fn = lua.create_function(move |lua, (style_val, content_val): (LuaValue, LuaValue)| {
            let style = value_to_style_prop(lua, style_val, &t)?;
            let content = value_to_string_prop(lua, content_val)?;
            Ok(LuaNode(Node::Text { style, content }))
        })?;
        lua.globals().set("text", text_fn)?;
    }

    // spacer() -> LuaNode
    {
        let spacer_fn = lua.create_function(|_lua, ()| Ok(LuaNode(Node::Spacer)))?;
        lua.globals().set("spacer", spacer_fn)?;
    }

    // button(style, opts, children?) -> LuaNode
    //
    // style: string or function (reactive)
    // opts: table with on_click, on_scroll_up, on_scroll_down, on_hover, on_hover_lost
    // children: optional table of child nodes
    {
        let t = theme.clone();
        let button_fn = lua.create_function(
            move |lua, (style_val, opts, children): (LuaValue, LuaTable, Option<LuaTable>)| {
                let style = value_to_style_prop(lua, style_val, &t)?;
                let nodes = match children {
                    Some(tbl) => table_to_nodes(&tbl)?,
                    None => Vec::new(),
                };
                let handlers = lua_table_to_handlers(lua, &opts)?;
                Ok(LuaNode(Node::Interactive {
                    style,
                    kind: InteractiveKind::Button { handlers },
                    children: nodes,
                }))
            },
        )?;
        lua.globals().set("button", button_fn)?;
    }

    // slider(style, opts) -> LuaNode
    //
    // opts:
    //   value     - LuaSignal (Signal<DynValue>) for the current numeric value
    //   on_change - function(v) called on interaction
    //   min       - number (default 0)
    //   max       - number (default 100)
    //
    // Style tokens: accent-<color> parsed manually for the filled track color.
    {
        let t = theme.clone();
        let slider_fn = lua.create_function(move |lua, (style_val, opts): (LuaValue, LuaTable)| {
            // Parse accent-* token out of style string before passing to the style prop.
            let (style, accent_color) = parse_style_with_accent(lua, style_val, &t)?;

            // Extract the value signal.
            let value_ud: LuaAnyUserData = opts.get("value")?;
            let value = value_ud.borrow::<LuaSignal>()?.0.clone();

            let min: f64 = opts.get::<Option<f64>>("min")?.unwrap_or(0.0);
            let max: f64 = opts.get::<Option<f64>>("max")?.unwrap_or(100.0);

            let on_change = wrap_lua_callback_f64(lua, opts.get::<Option<LuaFunction>>("on_change")?)?;

            Ok(LuaNode(Node::Interactive {
                style,
                kind: InteractiveKind::Slider {
                    value,
                    min,
                    max,
                    on_change,
                    accent_color,
                },
                children: Vec::new(),
            }))
        })?;
        lua.globals().set("slider", slider_fn)?;
    }

    // toggle(style, opts) -> LuaNode
    //
    // opts:
    //   checked   - LuaSignal (Signal<DynValue>) for the current boolean value
    //   on_change - function(v) called when toggled
    //
    // Style tokens: accent-<color> parsed manually for the track color when checked.
    {
        let t = theme.clone();
        let toggle_fn = lua.create_function(move |lua, (style_val, opts): (LuaValue, LuaTable)| {
            let (style, accent_color) = parse_style_with_accent(lua, style_val, &t)?;

            let checked_ud: LuaAnyUserData = opts.get("checked")?;
            let checked = checked_ud.borrow::<LuaSignal>()?.0.clone();

            let on_change = wrap_lua_callback_bool(lua, opts.get::<Option<LuaFunction>>("on_change")?)?;

            Ok(LuaNode(Node::Interactive {
                style,
                kind: InteractiveKind::Toggle {
                    checked,
                    on_change,
                    accent_color,
                },
                children: Vec::new(),
            }))
        })?;
        lua.globals().set("toggle", toggle_fn)?;
    }

    // each(items_fn, render_fn, key_fn?) -> LuaNode
    //
    // items_fn: function returning a Lua table (array of items)
    // render_fn: function(item) -> LuaNode
    // key_fn: optional function(item) -> string (stable identity for reconciliation)
    //
    // Returns a DynamicList node. On each layout pass, items_fn is called to get
    // the current items, render_fn produces nodes, and key_fn enables caching.
    {
        let _t = theme.clone();
        let each_fn = lua.create_function(
            move |lua, (items_fn, render_fn, key_fn, dir): (LuaFunction, LuaFunction, Option<LuaFunction>, Option<String>)| {
                let direction = match dir.as_deref() {
                    Some("row") => Direction::Row,
                    _ => Direction::Column, // default vertical for lists
                };
                let items_key = lua.create_registry_value(items_fn)?;
                let render_key = lua.create_registry_value(render_fn)?;
                let key_key = key_fn
                    .map(|f| lua.create_registry_value(f))
                    .transpose()?;

                let lua_clone = lua.clone();
                let cache: Rc<RefCell<Vec<(String, Node)>>> = Rc::new(RefCell::new(Vec::new()));
                let cached_children: Rc<RefCell<Vec<Node>>> = Rc::new(RefCell::new(Vec::new()));

                let resolve_cache = cache.clone();
                let resolve = Rc::new(move || -> Vec<Node> {
                    let items_fn: LuaFunction = lua_clone.registry_value(&items_key).unwrap();
                    let render_fn: LuaFunction = lua_clone.registry_value(&render_key).unwrap();

                    let items_table: LuaTable = match items_fn.call(()) {
                        Ok(t) => t,
                        Err(e) => {
                            log::error!("each() items_fn error: {e}");
                            return Vec::new();
                        }
                    };

                    let mut new_children = Vec::new();
                    let mut old_cache = resolve_cache.borrow_mut();
                    let mut new_cache = Vec::new();

                    for (i, item) in items_table.sequence_values::<LuaValue>().enumerate() {
                        let item = match item {
                            Ok(v) => v,
                            Err(e) => {
                                log::error!("each() item error at index {i}: {e}");
                                continue;
                            }
                        };

                        // Compute key: use key_fn if provided, else use index.
                        let key = if let Some(ref kk) = key_key {
                            let kf: LuaFunction = lua_clone.registry_value(kk).unwrap();
                            kf.call::<String>(item.clone()).unwrap_or_else(|_| i.to_string())
                        } else {
                            i.to_string()
                        };

                        // Check cache for existing node with this key.
                        if let Some(pos) = old_cache.iter().position(|(k, _)| k == &key) {
                            let (k, node) = old_cache.remove(pos);
                            new_children.push(node.clone());
                            new_cache.push((k, node));
                        } else {
                            // Cache miss — render new node.
                            match render_fn.call::<LuaAnyUserData>(item) {
                                Ok(ud) => {
                                    if let Ok(lua_node) = ud.borrow::<LuaNode>() {
                                        let node = lua_node.0.clone();
                                        new_children.push(node.clone());
                                        new_cache.push((key, node));
                                    }
                                }
                                Err(e) => {
                                    log::error!("each() render_fn error: {e}");
                                }
                            }
                        }
                    }

                    *old_cache = new_cache;
                    new_children
                });

                Ok(LuaNode(Node::DynamicList {
                    style: Prop::Static(StyleProps::default()),
                    direction,
                    resolve,
                    cached_children,
                }))
            },
        )?;
        lua.globals().set("each", each_fn)?;
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Parse a style value, extracting `accent-<color>` tokens before building the
/// `Prop<StyleProps>`. Returns (style_prop, accent_color).
///
/// For static strings, accent tokens are stripped before parsing.
/// For functions, accent parsing is deferred into the reactive closure.
fn parse_style_with_accent(
    lua: &Lua,
    val: LuaValue,
    theme: &Arc<Theme>,
) -> LuaResult<(Prop<StyleProps>, Option<Color>)> {
    match val {
        LuaValue::String(s) => {
            let raw = s.to_string_lossy();
            let (filtered, accent) = extract_accent(&raw, theme);
            let parsed = StyleProps::parse(&filtered, theme);
            Ok((Prop::Static(parsed), accent))
        }
        LuaValue::Function(f) => {
            let key = lua.create_registry_value(f)?;
            let lua_clone = lua.clone();
            let theme = theme.clone();
            let cached_str = RefCell::new(String::new());
            let cached_props = RefCell::new(StyleProps::default());
            // For reactive styles on slider/toggle, we cannot return a changing
            // accent color without making it Prop<Option<Color>> too, which is
            // overkill. We resolve the accent once on first call and keep it.
            let accent_cell: Rc<RefCell<Option<Color>>> = Rc::new(RefCell::new(None));
            let accent_out = accent_cell.clone();
            let prop = Prop::Reactive(Rc::new(move || {
                let cb: LuaFunction = lua_clone.registry_value(&key).unwrap();
                let new_str: String = cb.call(()).unwrap_or_default();
                if new_str != *cached_str.borrow() {
                    let (filtered, accent) = extract_accent(&new_str, &theme);
                    *cached_str.borrow_mut() = new_str;
                    *cached_props.borrow_mut() = StyleProps::parse(&filtered, &theme);
                    *accent_cell.borrow_mut() = accent;
                }
                cached_props.borrow().clone()
            }));
            // Resolve once now so the accent is available immediately.
            let _ = prop.resolve();
            let accent = accent_out.borrow().clone();
            Ok((prop, accent))
        }
        _ => Ok((Prop::Static(StyleProps::default()), None)),
    }
}

/// Strip `accent-<color>` tokens from a style string. Returns the filtered
/// string and the resolved accent color (if any).
fn extract_accent(raw: &str, theme: &Theme) -> (String, Option<Color>) {
    let mut accent = None;
    let mut filtered = Vec::new();
    for token in raw.split_whitespace() {
        if let Some(color_name) = token.strip_prefix("accent-") {
            if let Some(&c) = theme.colors.get(color_name) {
                accent = Some(c);
            }
        } else {
            filtered.push(token);
        }
    }
    (filtered.join(" "), accent)
}

/// Extract event-handler Lua functions from an options table and wrap them
/// as `EventHandlers` with `Rc`-wrapped closures.
fn lua_table_to_handlers(lua: &Lua, opts: &LuaTable) -> LuaResult<EventHandlers> {
    let on_click = wrap_lua_callback(lua, opts.get::<Option<LuaFunction>>("on_click")?)?;
    let on_scroll_up = wrap_lua_callback(lua, opts.get::<Option<LuaFunction>>("on_scroll_up")?)?;
    let on_scroll_down = wrap_lua_callback(lua, opts.get::<Option<LuaFunction>>("on_scroll_down")?)?;
    let on_hover = wrap_lua_callback(lua, opts.get::<Option<LuaFunction>>("on_hover")?)?;
    let on_hover_lost = wrap_lua_callback(lua, opts.get::<Option<LuaFunction>>("on_hover_lost")?)?;

    Ok(EventHandlers {
        on_click,
        on_scroll_up,
        on_scroll_down,
        on_hover,
        on_hover_lost,
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
