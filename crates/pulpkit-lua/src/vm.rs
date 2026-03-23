//! Lua VM wrapper — creates and manages a LuaJIT instance.

use std::path::Path;

use mlua::prelude::*;

/// Wrapper around a Lua VM instance.
pub struct LuaVm {
    lua: Lua,
}

impl LuaVm {
    /// Create a new Lua VM with LuaJIT.
    pub fn new() -> LuaResult<Self> {
        let lua = Lua::new();
        // Set up package.path to include the shell directory (set later via shell_dir global)
        Ok(LuaVm { lua })
    }

    /// Get a reference to the Lua state.
    pub fn lua(&self) -> &Lua {
        &self.lua
    }

    /// Load and execute a Lua file.
    pub fn load_file(&self, path: &Path) -> LuaResult<()> {
        let code = std::fs::read_to_string(path)
            .map_err(|e| LuaError::ExternalError(std::sync::Arc::new(e)))?;
        self.lua.load(&code).set_name(path.to_string_lossy()).exec()
    }
}
