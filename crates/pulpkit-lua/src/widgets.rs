use mlua::prelude::*;
use pulpkit_layout::tree::{Direction, Node};
use pulpkit_layout::style::StyleProps;
use pulpkit_layout::Theme;
use std::sync::Arc;

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

    Ok(())
}
