//! Widget constructor Lua globals: row, col, text, button, slider, toggle, etc.

use std::sync::Arc;

use mlua::prelude::*;

use pulpkit_layout::element::{Direction, Element, KeyedChild, MonitorTarget, SurfaceDef, SurfaceKind};
use pulpkit_layout::style::{self, StyleProps};
use pulpkit_layout::Theme;
use pulpkit_render::Color;

use crate::element::{LuaElement, collect_children, lua_to_element};
use crate::msg::{is_msg_table, lua_table_to_message};

/// Register all widget constructors as Lua globals.
pub fn register_widgets(lua: &Lua, theme: Arc<Theme>) -> LuaResult<()> {
    // row(opts, ...)
    let t = theme.clone();
    lua.globals().set("row", lua.create_function(move |_lua, args: LuaMultiValue| {
        build_container(&t, Direction::Row, args)
    })?)?;

    // col(opts, ...)
    let t = theme.clone();
    lua.globals().set("col", lua.create_function(move |_lua, args: LuaMultiValue| {
        build_container(&t, Direction::Column, args)
    })?)?;

    // text(opts_or_string, string?)
    let t = theme.clone();
    lua.globals().set("text", lua.create_function(move |_lua, args: LuaMultiValue| {
        build_text(&t, args)
    })?)?;

    // icon(name) — alias for text with icon styling
    lua.globals().set("icon", lua.create_function(|_lua, name: String| {
        Ok(LuaElement(Element::Text {
            style: StyleProps::default(),
            content: name,
        }))
    })?)?;

    // spacer()
    lua.globals().set("spacer", lua.create_function(|_lua, _: ()| {
        Ok(LuaElement(Element::Spacer))
    })?)?;

    // button(opts, ...)
    let t = theme.clone();
    lua.globals().set("button", lua.create_function(move |_lua, args: LuaMultiValue| {
        build_button(&t, args)
    })?)?;

    // slider(opts)
    let t = theme.clone();
    lua.globals().set("slider", lua.create_function(move |_lua, opts: LuaTable| {
        build_slider(&t, &opts)
    })?)?;

    // toggle(opts)
    let t = theme.clone();
    lua.globals().set("toggle", lua.create_function(move |_lua, opts: LuaTable| {
        build_toggle(&t, &opts)
    })?)?;

    // input(opts)
    let t = theme.clone();
    lua.globals().set("input", lua.create_function(move |_lua, opts: LuaTable| {
        build_input(&t, &opts)
    })?)?;

    // each(list, key_field, render_fn)
    let t = theme.clone();
    lua.globals().set("each", lua.create_function(move |lua, (list, key_field, render_fn): (LuaTable, String, LuaFunction)| {
        build_each(&t, lua, &list, &key_field, &render_fn)
    })?)?;

    // scroll(opts, ...)
    let t = theme.clone();
    lua.globals().set("scroll", lua.create_function(move |_lua, args: LuaMultiValue| {
        build_scroll(&t, args)
    })?)?;

    // image(path, opts)
    lua.globals().set("image", lua.create_function(|_lua, (path, opts): (String, Option<LuaTable>)| {
        let (width, height) = if let Some(ref opts) = opts {
            (opts.get::<f32>("width").unwrap_or(16.0), opts.get::<f32>("height").unwrap_or(16.0))
        } else {
            (16.0, 16.0)
        };
        Ok(LuaElement(Element::Image {
            style: StyleProps::default(),
            path,
            width,
            height,
        }))
    })?)?;

    // window(name, opts, child) — returns a tagged table for view() surfaces
    lua.globals().set("window", lua.create_function(|lua, (name, opts, child): (String, LuaTable, LuaValue)| {
        build_surface(lua, name, SurfaceKind::Window, &opts, &child)
    })?)?;

    // popup(name, opts, child) — returns a tagged table for view() surfaces
    lua.globals().set("popup", lua.create_function(|lua, (name, opts, child): (String, LuaTable, LuaValue)| {
        build_surface(lua, name, SurfaceKind::Popup, &opts, &child)
    })?)?;

    Ok(())
}

fn get_style(opts: &LuaTable, theme: &Theme) -> (StyleProps, Option<StyleProps>) {
    let style_str: String = opts.get("style").unwrap_or_default();
    style::parse_with_hover(&style_str, theme)
}

fn get_msg(opts: &LuaTable, key: &str) -> LuaResult<Option<pulpkit_layout::element::Message>> {
    let val: LuaValue = opts.get(key)?;
    match val {
        LuaValue::Table(ref t) if is_msg_table(t) => Ok(Some(lua_table_to_message(t)?)),
        LuaValue::Nil => Ok(None),
        _ => Ok(None),
    }
}

fn build_container(theme: &Theme, direction: Direction, args: LuaMultiValue) -> LuaResult<LuaElement> {
    let mut iter = args.into_iter();
    let opts = match iter.next() {
        Some(LuaValue::Table(t)) => t,
        _ => return Err(LuaError::RuntimeError("row/col first arg must be opts table".into())),
    };
    let (base_style, hover_style) = get_style(&opts, theme);
    let rest: Vec<LuaValue> = iter.collect();
    let children = collect_children(&rest)?;

    Ok(LuaElement(Element::Container {
        style: base_style,
        hover_style,
        direction,
        children,
    }))
}

fn build_text(theme: &Theme, args: LuaMultiValue) -> LuaResult<LuaElement> {
    let values: Vec<LuaValue> = args.into_iter().collect();
    match values.len() {
        1 => {
            // text("content") or text(opts_table)
            match &values[0] {
                LuaValue::String(s) => Ok(LuaElement(Element::Text {
                    style: StyleProps::default(),
                    content: s.to_str()?.to_string(),
                })),
                LuaValue::Table(opts) => {
                    let (style, _) = get_style(opts, theme);
                    let content: String = opts.get("content").unwrap_or_default();
                    Ok(LuaElement(Element::Text { style, content }))
                }
                _ => Err(LuaError::RuntimeError("text() requires string or opts table".into())),
            }
        }
        2 => {
            // text(opts, "content")
            let opts = match &values[0] {
                LuaValue::Table(t) => t,
                _ => return Err(LuaError::RuntimeError("text() first arg must be opts table".into())),
            };
            let content = match &values[1] {
                LuaValue::String(s) => s.to_str()?.to_string(),
                LuaValue::Number(n) => n.to_string(),
                LuaValue::Integer(i) => i.to_string(),
                _ => return Err(LuaError::RuntimeError("text() second arg must be string".into())),
            };
            let (style, _) = get_style(opts, theme);
            Ok(LuaElement(Element::Text { style, content }))
        }
        _ => Err(LuaError::RuntimeError("text() takes 1 or 2 arguments".into())),
    }
}

fn build_button(theme: &Theme, args: LuaMultiValue) -> LuaResult<LuaElement> {
    let mut iter = args.into_iter();
    let opts = match iter.next() {
        Some(LuaValue::Table(t)) => t,
        _ => return Err(LuaError::RuntimeError("button first arg must be opts table".into())),
    };
    let (base_style, hover_style) = get_style(&opts, theme);
    let on_click = get_msg(&opts, "on_click")?;
    let on_hover = get_msg(&opts, "on_hover")?;
    let on_hover_lost = get_msg(&opts, "on_hover_lost")?;
    let rest: Vec<LuaValue> = iter.collect();
    let children = collect_children(&rest)?;

    Ok(LuaElement(Element::Button {
        style: base_style,
        hover_style,
        on_click,
        on_hover,
        on_hover_lost,
        children,
    }))
}

fn build_slider(theme: &Theme, opts: &LuaTable) -> LuaResult<LuaElement> {
    let (style, _) = get_style(opts, theme);
    let value: f64 = opts.get("value").unwrap_or(0.0);
    let min: f64 = opts.get("min").unwrap_or(0.0);
    let max: f64 = opts.get("max").unwrap_or(100.0);
    let on_change = get_msg(opts, "on_change")?;
    let accent_hex: Option<String> = opts.get("accent").ok();
    let accent_color = accent_hex.and_then(|h| Color::from_hex(&h));

    Ok(LuaElement(Element::Slider {
        style, value, min, max, on_change, accent_color,
    }))
}

fn build_toggle(theme: &Theme, opts: &LuaTable) -> LuaResult<LuaElement> {
    let (style, _) = get_style(opts, theme);
    let checked: bool = opts.get("checked").unwrap_or(false);
    let on_toggle = get_msg(opts, "on_toggle")?;
    let accent_hex: Option<String> = opts.get("accent").ok();
    let accent_color = accent_hex.and_then(|h| Color::from_hex(&h));

    Ok(LuaElement(Element::Toggle {
        style, checked, on_toggle, accent_color,
    }))
}

fn build_input(theme: &Theme, opts: &LuaTable) -> LuaResult<LuaElement> {
    let (style, _) = get_style(opts, theme);
    let value: String = opts.get("value").unwrap_or_default();
    let placeholder: String = opts.get("placeholder").unwrap_or_default();
    let on_input = get_msg(opts, "on_input")?;

    Ok(LuaElement(Element::Input {
        style, value, placeholder, on_input,
    }))
}

fn build_each(
    _theme: &Theme,
    lua: &Lua,
    list: &LuaTable,
    key_field: &str,
    render_fn: &LuaFunction,
) -> LuaResult<LuaElement> {
    let mut children = Vec::new();
    for pair in list.sequence_values::<LuaTable>() {
        let item = pair?;
        let key: String = item.get(key_field)?;
        let result = render_fn.call::<LuaValue>(item)?;
        let element = lua_to_element(&result)?;
        children.push(KeyedChild { key, element });
    }

    Ok(LuaElement(Element::Each {
        style: StyleProps::default(),
        direction: Direction::Row,
        children,
    }))
}

fn build_scroll(theme: &Theme, args: LuaMultiValue) -> LuaResult<LuaElement> {
    let mut iter = args.into_iter();
    let opts = match iter.next() {
        Some(LuaValue::Table(t)) => t,
        _ => return Err(LuaError::RuntimeError("scroll first arg must be opts table".into())),
    };
    let (style, _) = get_style(&opts, theme);
    let rest: Vec<LuaValue> = iter.collect();
    let children = collect_children(&rest)?;

    Ok(LuaElement(Element::Scroll {
        style,
        children,
        scroll_offset: 0.0,
    }))
}

fn build_surface(
    lua: &Lua,
    name: String,
    kind: SurfaceKind,
    opts: &LuaTable,
    child: &LuaValue,
) -> LuaResult<LuaTable> {
    let root = lua_to_element(child)?;
    let anchor: String = opts.get("anchor").unwrap_or_else(|_| "top".into());
    let width: Option<u32> = opts.get("width").ok();
    let height: Option<u32> = opts.get("height").ok();
    let exclusive: bool = opts.get("exclusive").unwrap_or(false);
    let dismiss_on_outside: bool = opts.get("dismiss_on_outside").unwrap_or(false);
    let monitor_str: String = opts.get("monitor").unwrap_or_else(|_| "primary".into());
    let monitor = match monitor_str.as_str() {
        "all" => MonitorTarget::All,
        "primary" => MonitorTarget::Primary,
        other => MonitorTarget::Named(other.into()),
    };

    // Store the SurfaceDef as a userdata in the table
    let table = lua.create_table()?;
    table.set("__pulpkit_surface", true)?;
    table.set("name", name.as_str())?;
    table.set("kind", match kind { SurfaceKind::Window => "window", SurfaceKind::Popup => "popup" })?;
    // Store the root element as userdata
    table.set("root", LuaElement(root))?;
    table.set("anchor", anchor.as_str())?;
    if let Some(w) = width { table.set("width", w)?; }
    if let Some(h) = height { table.set("height", h)?; }
    table.set("exclusive", exclusive)?;
    table.set("dismiss_on_outside", dismiss_on_outside)?;
    table.set("monitor", monitor_str.as_str())?;

    Ok(table)
}

/// Extract a SurfaceDef from a Lua surface table (returned by window/popup).
pub fn lua_table_to_surface_def(table: &LuaTable) -> LuaResult<SurfaceDef> {
    let name: String = table.get("name")?;
    let kind_str: String = table.get("kind")?;
    let kind = match kind_str.as_str() {
        "popup" => SurfaceKind::Popup,
        _ => SurfaceKind::Window,
    };
    let root_ud: LuaAnyUserData = table.get("root")?;
    let root = root_ud.borrow::<LuaElement>()?.0.clone();
    let anchor: String = table.get("anchor")?;
    let width: Option<u32> = table.get("width").ok();
    let height: Option<u32> = table.get("height").ok();
    let exclusive: bool = table.get("exclusive").unwrap_or(false);
    let dismiss_on_outside: bool = table.get("dismiss_on_outside").unwrap_or(false);
    let monitor_str: String = table.get("monitor").unwrap_or_else(|_| "primary".into());
    let monitor = match monitor_str.as_str() {
        "all" => MonitorTarget::All,
        "primary" => MonitorTarget::Primary,
        other => MonitorTarget::Named(other.into()),
    };

    Ok(SurfaceDef {
        name, kind, anchor, width, height, exclusive, monitor, dismiss_on_outside, root,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> (Lua, Arc<Theme>) {
        let lua = Lua::new();
        let theme = Arc::new(Theme::default_slate());
        crate::register_msg_api(&lua).unwrap();
        register_widgets(&lua, theme.clone()).unwrap();
        (lua, theme)
    }

    #[test]
    fn row_with_text_children() {
        let (lua, _) = setup();
        lua.load(r#"result = row({ style = "bg-base p-2" }, text("hello"), text("world"))"#).exec().unwrap();
        let ud: LuaAnyUserData = lua.globals().get("result").unwrap();
        let el = ud.borrow::<LuaElement>().unwrap();
        match &el.0 {
            Element::Container { children, direction, .. } => {
                assert_eq!(children.len(), 2);
                assert_eq!(*direction, Direction::Row);
            }
            _ => panic!("expected Container"),
        }
    }

    #[test]
    fn button_with_msg_and_hover() {
        let (lua, _) = setup();
        lua.load(r#"
            result = button({ on_click = msg("open"), style = "hover:bg-surface p-2" },
                text("Go"))
        "#).exec().unwrap();
        let ud: LuaAnyUserData = lua.globals().get("result").unwrap();
        let el = ud.borrow::<LuaElement>().unwrap();
        match &el.0 {
            Element::Button { on_click, hover_style, children, .. } => {
                assert!(on_click.is_some());
                assert_eq!(on_click.as_ref().unwrap().msg_type, "open");
                assert!(hover_style.is_some());
                assert_eq!(children.len(), 1);
            }
            _ => panic!("expected Button"),
        }
    }

    #[test]
    fn slider_creation() {
        let (lua, _) = setup();
        lua.load(r#"result = slider({ value = 50, min = 0, max = 100, on_change = msg("vol") })"#).exec().unwrap();
        let ud: LuaAnyUserData = lua.globals().get("result").unwrap();
        let el = ud.borrow::<LuaElement>().unwrap();
        match &el.0 {
            Element::Slider { value, min, max, on_change, .. } => {
                assert_eq!(*value, 50.0);
                assert_eq!(*min, 0.0);
                assert_eq!(*max, 100.0);
                assert!(on_change.is_some());
            }
            _ => panic!("expected Slider"),
        }
    }

    #[test]
    fn each_with_keyed_list() {
        let (lua, _) = setup();
        lua.load(r#"
            local items = {
                { id = "a", label = "Alpha" },
                { id = "b", label = "Beta" },
                { id = "c", label = "Gamma" },
            }
            result = each(items, "id", function(item)
                return text(item.label)
            end)
        "#).exec().unwrap();
        let ud: LuaAnyUserData = lua.globals().get("result").unwrap();
        let el = ud.borrow::<LuaElement>().unwrap();
        match &el.0 {
            Element::Each { children, .. } => {
                assert_eq!(children.len(), 3);
                assert_eq!(children[0].key, "a");
                assert_eq!(children[1].key, "b");
                assert_eq!(children[2].key, "c");
            }
            _ => panic!("expected Each"),
        }
    }

    #[test]
    fn window_surface_def() {
        let (lua, _) = setup();
        lua.load(r#"
            result = window("bar", { anchor = "top", height = 40, exclusive = true, monitor = "all" },
                row({ style = "bg-base" }, text("hello")))
        "#).exec().unwrap();
        let table: LuaTable = lua.globals().get("result").unwrap();
        let def = lua_table_to_surface_def(&table).unwrap();
        assert_eq!(def.name, "bar");
        assert_eq!(def.kind, SurfaceKind::Window);
        assert_eq!(def.anchor, "top");
        assert_eq!(def.height, Some(40));
        assert!(def.exclusive);
        assert_eq!(def.monitor, MonitorTarget::All);
    }

    #[test]
    fn toggle_creation() {
        let (lua, _) = setup();
        lua.load(r#"result = toggle({ checked = true, on_toggle = msg("dark") })"#).exec().unwrap();
        let ud: LuaAnyUserData = lua.globals().get("result").unwrap();
        let el = ud.borrow::<LuaElement>().unwrap();
        match &el.0 {
            Element::Toggle { checked, on_toggle, .. } => {
                assert!(*checked);
                assert!(on_toggle.is_some());
            }
            _ => panic!("expected Toggle"),
        }
    }
}
