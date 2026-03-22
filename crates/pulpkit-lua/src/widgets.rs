use std::rc::Rc;
use std::sync::Arc;

use mlua::prelude::*;
use pulpkit_layout::tree::{ButtonHandlers, Direction, Node};
use pulpkit_layout::style::StyleProps;
use pulpkit_layout::Theme;

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
