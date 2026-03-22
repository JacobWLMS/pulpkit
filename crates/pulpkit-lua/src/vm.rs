use mlua::prelude::*;
use std::path::Path;

pub struct LuaVm {
    lua: Lua,
}

impl LuaVm {
    pub fn new() -> LuaResult<Self> {
        let lua = Lua::new();
        Ok(Self { lua })
    }

    pub fn lua(&self) -> &Lua {
        &self.lua
    }

    /// Load and execute a Lua file.
    pub fn load_file(&self, path: &Path) -> LuaResult<()> {
        let code = std::fs::read_to_string(path).map_err(LuaError::external)?;
        self.lua
            .load(&code)
            .set_name(path.to_string_lossy())
            .exec()
    }

    /// Clear Lua's module cache so require() reloads files.
    pub fn clear_module_cache(&self) -> LuaResult<()> {
        self.lua.load(r#"
            for k, _ in pairs(package.loaded) do
                package.loaded[k] = nil
            end
        "#).exec()
    }
}
