//! msg() API — creates inert message values for Elm-style event handling.

use mlua::prelude::*;

use pulpkit_layout::element::{Message, MessageData};

/// Register the `msg(name, data?)` global function.
pub fn register_msg_api(lua: &Lua) -> LuaResult<()> {
    let msg_fn = lua.create_function(|lua, args: LuaMultiValue| {
        let mut iter = args.into_iter();
        let name: String = match iter.next() {
            Some(LuaValue::String(s)) => s.to_str()?.to_string(),
            _ => return Err(LuaError::RuntimeError("msg() requires a string name".into())),
        };

        let data = iter.next();

        let table = lua.create_table()?;
        table.set("type", name)?;
        if let Some(val) = data {
            table.set("data", val)?;
        }

        // Tag with metatable so we can identify msg tables
        let mt = lua.create_table()?;
        mt.set("__pulpkit_msg", true)?;
        let _ = table.set_metatable(Some(mt));

        Ok(table)
    })?;

    lua.globals().set("msg", msg_fn)?;
    Ok(())
}

/// Check if a Lua table is a msg() value (has __pulpkit_msg metatable).
pub fn is_msg_table(table: &LuaTable) -> bool {
    table.metatable()
        .and_then(|mt| mt.get::<bool>("__pulpkit_msg").ok())
        .unwrap_or(false)
}

/// Convert a Lua msg table to a Rust Message.
pub fn lua_table_to_message(table: &LuaTable) -> LuaResult<Message> {
    let msg_type: String = table.get("type")?;
    let data: LuaValue = table.get("data")?;
    let data = lua_value_to_message_data(&data);
    Ok(Message { msg_type, data })
}

/// Convert a Lua value to MessageData.
pub fn lua_value_to_message_data(val: &LuaValue) -> Option<MessageData> {
    match val {
        LuaValue::Nil => None,
        LuaValue::Boolean(b) => Some(MessageData::Bool(*b)),
        LuaValue::Integer(i) => Some(MessageData::Int(*i as i64)),
        LuaValue::Number(n) => Some(MessageData::Float(*n)),
        LuaValue::String(s) => Some(MessageData::String(s.to_str().map(|s| s.to_string()).unwrap_or_default())),
        LuaValue::Table(t) => {
            let mut entries = Vec::new();
            for pair in t.pairs::<String, LuaValue>() {
                if let Ok((k, v)) = pair {
                    if let Some(data) = lua_value_to_message_data(&v) {
                        entries.push((k, data));
                    }
                }
            }
            if entries.is_empty() {
                None
            } else {
                Some(MessageData::Table(entries))
            }
        }
        _ => None,
    }
}

/// Convert a Rust Message to a Lua table.
pub fn message_to_lua_table(lua: &Lua, msg: &Message) -> LuaResult<LuaTable> {
    let table = lua.create_table()?;
    table.set("type", msg.msg_type.as_str())?;
    if let Some(ref data) = msg.data {
        table.set("data", message_data_to_lua(lua, data)?)?;
    }
    Ok(table)
}

fn message_data_to_lua(lua: &Lua, data: &MessageData) -> LuaResult<LuaValue> {
    match data {
        MessageData::String(s) => Ok(LuaValue::String(lua.create_string(s)?)),
        MessageData::Float(f) => Ok(LuaValue::Number(*f)),
        MessageData::Bool(b) => Ok(LuaValue::Boolean(*b)),
        MessageData::Int(i) => Ok(LuaValue::Integer(*i as _)),
        MessageData::Table(entries) => {
            let t = lua.create_table()?;
            for (k, v) in entries {
                t.set(k.as_str(), message_data_to_lua(lua, v)?)?;
            }
            Ok(LuaValue::Table(t))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn msg_simple() {
        let lua = Lua::new();
        register_msg_api(&lua).unwrap();
        lua.load(r#"result = msg("click")"#).exec().unwrap();
        let table: LuaTable = lua.globals().get("result").unwrap();
        assert!(is_msg_table(&table));
        let msg = lua_table_to_message(&table).unwrap();
        assert_eq!(msg.msg_type, "click");
        assert!(msg.data.is_none());
    }

    #[test]
    fn msg_with_number_data() {
        let lua = Lua::new();
        register_msg_api(&lua).unwrap();
        lua.load(r#"result = msg("set_vol", 75)"#).exec().unwrap();
        let table: LuaTable = lua.globals().get("result").unwrap();
        let msg = lua_table_to_message(&table).unwrap();
        assert_eq!(msg.msg_type, "set_vol");
        assert_eq!(msg.data.unwrap().as_f64(), Some(75.0));
    }

    #[test]
    fn msg_with_table_data() {
        let lua = Lua::new();
        register_msg_api(&lua).unwrap();
        lua.load(r#"result = msg("scroll", { delta = -1 })"#).exec().unwrap();
        let table: LuaTable = lua.globals().get("result").unwrap();
        let msg = lua_table_to_message(&table).unwrap();
        assert_eq!(msg.msg_type, "scroll");
        match msg.data.unwrap() {
            MessageData::Table(entries) => {
                assert_eq!(entries.len(), 1);
                assert_eq!(entries[0].0, "delta");
            }
            other => panic!("expected Table, got {:?}", other),
        }
    }

    #[test]
    fn msg_with_string_data() {
        let lua = Lua::new();
        register_msg_api(&lua).unwrap();
        lua.load(r#"result = msg("hover", "power")"#).exec().unwrap();
        let table: LuaTable = lua.globals().get("result").unwrap();
        let msg = lua_table_to_message(&table).unwrap();
        assert_eq!(msg.data.unwrap().as_str(), Some("power"));
    }
}
