//! Elm bridge — calls init/update/view/subscribe lifecycle functions in Lua.

use std::path::Path;

use mlua::prelude::*;

use pulpkit_layout::element::{Message, SurfaceDef};

use crate::msg::message_to_lua_table;
use crate::subscribe::{SubscriptionDef, parse_subscriptions};
use crate::widgets::lua_table_to_surface_def;

/// Bridge to the Elm lifecycle functions in Lua.
pub struct ElmBridge {
    init_key: LuaRegistryKey,
    update_key: LuaRegistryKey,
    view_key: LuaRegistryKey,
    subscribe_key: LuaRegistryKey,
    state_key: LuaRegistryKey,
}

impl ElmBridge {
    /// Load shell.lua and extract the four lifecycle functions.
    pub fn load(lua: &Lua, shell_path: &Path) -> LuaResult<Self> {
        let code = std::fs::read_to_string(shell_path)
            .map_err(|e| LuaError::ExternalError(std::sync::Arc::new(e)))?;
        lua.load(&code).set_name(shell_path.to_string_lossy()).exec()?;

        let init: LuaFunction = lua.globals().get("init")?;
        let update: LuaFunction = lua.globals().get("update")?;
        let view: LuaFunction = lua.globals().get("view")?;
        let subscribe: LuaFunction = lua.globals().get("subscribe")?;

        let init_key = lua.create_registry_value(init)?;
        let update_key = lua.create_registry_value(update)?;
        let view_key = lua.create_registry_value(view)?;
        let subscribe_key = lua.create_registry_value(subscribe)?;

        // Placeholder state — will be set by init()
        let state_key = lua.create_registry_value(LuaValue::Nil)?;

        Ok(ElmBridge {
            init_key,
            update_key,
            view_key,
            subscribe_key,
            state_key,
        })
    }

    /// Load from a Lua code string (for testing).
    pub fn load_string(lua: &Lua, code: &str) -> LuaResult<Self> {
        lua.load(code).exec()?;

        let init: LuaFunction = lua.globals().get("init")?;
        let update: LuaFunction = lua.globals().get("update")?;
        let view: LuaFunction = lua.globals().get("view")?;
        let subscribe: LuaFunction = lua.globals().get("subscribe")?;

        let init_key = lua.create_registry_value(init)?;
        let update_key = lua.create_registry_value(update)?;
        let view_key = lua.create_registry_value(view)?;
        let subscribe_key = lua.create_registry_value(subscribe)?;
        let state_key = lua.create_registry_value(LuaValue::Nil)?;

        Ok(ElmBridge {
            init_key, update_key, view_key, subscribe_key, state_key,
        })
    }

    /// Call init() and store the returned state table.
    pub fn init(&mut self, lua: &Lua) -> LuaResult<()> {
        let init_fn: LuaFunction = lua.registry_value(&self.init_key)?;
        let state: LuaTable = init_fn.call(())?;
        self.state_key = lua.create_registry_value(state)?;
        Ok(())
    }

    /// Call update(state, msg) for a single message.
    pub fn update(&mut self, lua: &Lua, msg: &Message) -> LuaResult<()> {
        let update_fn: LuaFunction = lua.registry_value(&self.update_key)?;
        let state: LuaTable = lua.registry_value(&self.state_key)?;
        let msg_table = message_to_lua_table(lua, msg)?;
        let new_state: LuaTable = update_fn.call((state, msg_table))?;
        self.state_key = lua.create_registry_value(new_state)?;
        Ok(())
    }

    /// Call view(state) and parse the returned surface list.
    pub fn view(&self, lua: &Lua) -> LuaResult<Vec<SurfaceDef>> {
        let view_fn: LuaFunction = lua.registry_value(&self.view_key)?;
        let state: LuaTable = lua.registry_value(&self.state_key)?;
        let result: LuaValue = view_fn.call(state)?;

        match result {
            LuaValue::Table(list) => {
                let mut surfaces = Vec::new();
                for val in list.sequence_values::<LuaTable>() {
                    let table = val?;
                    let def = lua_table_to_surface_def(&table)?;
                    surfaces.push(def);
                }
                Ok(surfaces)
            }
            _ => Err(LuaError::RuntimeError("view() must return a table of surfaces".into())),
        }
    }

    /// Call subscribe(state) and parse the returned subscription list.
    pub fn subscribe(&self, lua: &Lua) -> LuaResult<Vec<SubscriptionDef>> {
        let sub_fn: LuaFunction = lua.registry_value(&self.subscribe_key)?;
        let state: LuaTable = lua.registry_value(&self.state_key)?;
        let result: LuaTable = sub_fn.call(state)?;
        parse_subscriptions(&result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use pulpkit_layout::Theme;
    use crate::{register_msg_api, register_widgets};
    use crate::subscribe::register_subscribe_api;

    const TEST_SHELL: &str = r#"
        function init()
            return { count = 0 }
        end

        function update(state, msg)
            if msg.type == "inc" then
                state.count = state.count + 1
            end
            return state
        end

        function view(state)
            return {
                window("test", { anchor = "top", height = 40 },
                    text(tostring(state.count))
                )
            }
        end

        function subscribe(state)
            return { interval(1000, "tick") }
        end
    "#;

    fn setup_bridge() -> (Lua, ElmBridge) {
        let lua = Lua::new();
        let theme = Arc::new(Theme::default_slate());
        register_msg_api(&lua).unwrap();
        register_widgets(&lua, theme).unwrap();
        register_subscribe_api(&lua).unwrap();

        let mut bridge = ElmBridge::load_string(&lua, TEST_SHELL).unwrap();
        bridge.init(&lua).unwrap();
        (lua, bridge)
    }

    #[test]
    fn bridge_init_succeeds() {
        let _ = setup_bridge();
    }

    #[test]
    fn bridge_update_increments() {
        let (lua, mut bridge) = setup_bridge();
        let msg = Message { msg_type: "inc".into(), data: None };
        bridge.update(&lua, &msg).unwrap();
        // After incrementing, view should show "1"
        let surfaces = bridge.view(&lua).unwrap();
        assert_eq!(surfaces.len(), 1);
        assert_eq!(surfaces[0].name, "test");
    }

    #[test]
    fn bridge_view_returns_surfaces() {
        let (lua, bridge) = setup_bridge();
        let surfaces = bridge.view(&lua).unwrap();
        assert_eq!(surfaces.len(), 1);
        assert_eq!(surfaces[0].name, "test");
        assert_eq!(surfaces[0].anchor, "top");
        assert_eq!(surfaces[0].height, Some(40));
    }

    #[test]
    fn bridge_subscribe_returns_defs() {
        let (lua, bridge) = setup_bridge();
        let subs = bridge.subscribe(&lua).unwrap();
        assert_eq!(subs.len(), 1);
        assert_eq!(subs[0], SubscriptionDef::Interval { ms: 1000, msg_name: "tick".into() });
    }

    #[test]
    fn bridge_lua_error_in_update_is_recoverable() {
        let lua = Lua::new();
        let theme = Arc::new(Theme::default_slate());
        register_msg_api(&lua).unwrap();
        register_widgets(&lua, theme).unwrap();
        register_subscribe_api(&lua).unwrap();

        let code = r#"
            function init() return { crash = false } end
            function update(state, msg)
                if msg.type == "crash" then error("boom!") end
                return state
            end
            function view(state)
                return { window("test", { anchor = "top", height = 40 }, text("ok")) }
            end
            function subscribe(state) return {} end
        "#;

        let mut bridge = ElmBridge::load_string(&lua, code).unwrap();
        bridge.init(&lua).unwrap();

        // Normal update works
        let msg = Message { msg_type: "safe".into(), data: None };
        assert!(bridge.update(&lua, &msg).is_ok());

        // Crashing update returns Err (but bridge is still usable)
        let crash = Message { msg_type: "crash".into(), data: None };
        assert!(bridge.update(&lua, &crash).is_err());

        // Bridge recovers — view still works with previous state
        let surfaces = bridge.view(&lua).unwrap();
        assert_eq!(surfaces.len(), 1);
    }
}
