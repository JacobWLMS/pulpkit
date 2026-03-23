//! Subscription descriptors — parsed from Lua's subscribe() return value.

use mlua::prelude::*;

/// A subscription definition from Lua.
#[derive(Debug, Clone, PartialEq)]
pub enum SubscriptionDef {
    Interval { ms: u64, msg_name: String },
    Timeout { ms: u64, msg_name: String },
    Stream { cmd: String, msg_name: String },
    Exec { cmd: String, msg_name: String },
    ConfigWatch { path: String, msg_name: String },
    Dbus { bus: String, path: String, iface: String, signal: String, msg_name: String },
    Ipc { msg_name: String },
}

impl SubscriptionDef {
    /// Key for matching: (variant name, msg_name). Used by reconciliation.
    pub fn sub_key(&self) -> (&'static str, &str) {
        match self {
            SubscriptionDef::Interval { msg_name, .. } => ("interval", msg_name),
            SubscriptionDef::Timeout { msg_name, .. } => ("timeout", msg_name),
            SubscriptionDef::Stream { msg_name, .. } => ("stream", msg_name),
            SubscriptionDef::Exec { msg_name, .. } => ("exec", msg_name),
            SubscriptionDef::ConfigWatch { msg_name, .. } => ("config_watch", msg_name),
            SubscriptionDef::Dbus { msg_name, .. } => ("dbus", msg_name),
            SubscriptionDef::Ipc { msg_name, .. } => ("ipc", msg_name),
        }
    }
}

/// Register subscription constructor globals: interval, timeout, stream, exec, etc.
pub fn register_subscribe_api(lua: &Lua) -> LuaResult<()> {
    lua.globals().set("interval", lua.create_function(|lua, (ms, msg_name): (u64, String)| {
        let t = lua.create_table()?;
        t.set("__sub_type", "interval")?;
        t.set("ms", ms)?;
        t.set("msg", msg_name)?;
        Ok(t)
    })?)?;

    lua.globals().set("timeout", lua.create_function(|lua, (ms, msg_name): (u64, String)| {
        let t = lua.create_table()?;
        t.set("__sub_type", "timeout")?;
        t.set("ms", ms)?;
        t.set("msg", msg_name)?;
        Ok(t)
    })?)?;

    lua.globals().set("stream", lua.create_function(|lua, (cmd, msg_name): (String, String)| {
        let t = lua.create_table()?;
        t.set("__sub_type", "stream")?;
        t.set("cmd", cmd)?;
        t.set("msg", msg_name)?;
        Ok(t)
    })?)?;

    lua.globals().set("exec", lua.create_function(|lua, (cmd, msg_name): (String, String)| {
        let t = lua.create_table()?;
        t.set("__sub_type", "exec")?;
        t.set("cmd", cmd)?;
        t.set("msg", msg_name)?;
        Ok(t)
    })?)?;

    lua.globals().set("config_watch", lua.create_function(|lua, (path, msg_name): (String, String)| {
        let t = lua.create_table()?;
        t.set("__sub_type", "config_watch")?;
        t.set("path", path)?;
        t.set("msg", msg_name)?;
        Ok(t)
    })?)?;

    lua.globals().set("dbus", lua.create_function(|lua, (bus, path, iface, signal, msg_name): (String, String, String, String, String)| {
        let t = lua.create_table()?;
        t.set("__sub_type", "dbus")?;
        t.set("bus", bus)?;
        t.set("path", path)?;
        t.set("iface", iface)?;
        t.set("signal", signal)?;
        t.set("msg", msg_name)?;
        Ok(t)
    })?)?;

    lua.globals().set("ipc", lua.create_function(|lua, msg_name: String| {
        let t = lua.create_table()?;
        t.set("__sub_type", "ipc")?;
        t.set("msg", msg_name)?;
        Ok(t)
    })?)?;

    Ok(())
}

/// Parse a Lua table (list of sub descriptors) into SubscriptionDefs.
pub fn parse_subscriptions(table: &LuaTable) -> LuaResult<Vec<SubscriptionDef>> {
    let mut subs = Vec::new();
    for val in table.sequence_values::<LuaTable>() {
        let t = val?;
        let sub_type: String = t.get("__sub_type")?;
        let def = match sub_type.as_str() {
            "interval" => SubscriptionDef::Interval {
                ms: t.get("ms")?,
                msg_name: t.get("msg")?,
            },
            "timeout" => SubscriptionDef::Timeout {
                ms: t.get("ms")?,
                msg_name: t.get("msg")?,
            },
            "stream" => SubscriptionDef::Stream {
                cmd: t.get("cmd")?,
                msg_name: t.get("msg")?,
            },
            "exec" => SubscriptionDef::Exec {
                cmd: t.get("cmd")?,
                msg_name: t.get("msg")?,
            },
            "config_watch" => SubscriptionDef::ConfigWatch {
                path: t.get("path")?,
                msg_name: t.get("msg")?,
            },
            "dbus" => SubscriptionDef::Dbus {
                bus: t.get("bus")?,
                path: t.get("path")?,
                iface: t.get("iface")?,
                signal: t.get("signal")?,
                msg_name: t.get("msg")?,
            },
            "ipc" => SubscriptionDef::Ipc {
                msg_name: t.get("msg")?,
            },
            other => {
                log::warn!("Unknown subscription type: {other}");
                continue;
            }
        };
        subs.push(def);
    }
    Ok(subs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_interval_and_stream() {
        let lua = Lua::new();
        register_subscribe_api(&lua).unwrap();
        lua.load(r#"
            result = {
                interval(1000, "tick"),
                stream("pactl subscribe", "audio"),
                exec("whoami", "user"),
            }
        "#).exec().unwrap();
        let table: LuaTable = lua.globals().get("result").unwrap();
        let subs = parse_subscriptions(&table).unwrap();
        assert_eq!(subs.len(), 3);
        assert_eq!(subs[0], SubscriptionDef::Interval { ms: 1000, msg_name: "tick".into() });
        assert_eq!(subs[1], SubscriptionDef::Stream { cmd: "pactl subscribe".into(), msg_name: "audio".into() });
        assert_eq!(subs[2], SubscriptionDef::Exec { cmd: "whoami".into(), msg_name: "user".into() });
    }

    #[test]
    fn sub_key_matching() {
        let a = SubscriptionDef::Interval { ms: 1000, msg_name: "tick".into() };
        let b = SubscriptionDef::Interval { ms: 2000, msg_name: "tick".into() };
        assert_eq!(a.sub_key(), b.sub_key()); // same key, different ms
        let c = SubscriptionDef::Interval { ms: 1000, msg_name: "other".into() };
        assert_ne!(a.sub_key(), c.sub_key()); // different msg_name
    }
}
