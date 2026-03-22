//! System interaction — `exec()`, `exec_output()`, and `env()` for Lua.

use mlua::prelude::*;
use std::process::Command;

/// Register system interaction functions as Lua globals.
pub fn register_system_api(lua: &Lua) -> LuaResult<()> {
    // exec(cmd) — run a shell command asynchronously (fire-and-forget).
    // Returns immediately. Output is discarded.
    let exec_fn = lua.create_function(|_lua, cmd: String| {
        std::thread::spawn(move || {
            let _ = Command::new("sh").arg("-c").arg(&cmd).output();
        });
        Ok(())
    })?;
    lua.globals().set("exec", exec_fn)?;

    // exec_output(cmd) — run a shell command and return its stdout.
    // Blocks until the command completes. Returns trimmed stdout string.
    let exec_output_fn = lua.create_function(|_lua, cmd: String| {
        match Command::new("sh").arg("-c").arg(&cmd).output() {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
                Ok(stdout)
            }
            Err(e) => {
                log::error!("exec_output failed: {e}");
                Ok(String::new())
            }
        }
    })?;
    lua.globals().set("exec_output", exec_output_fn)?;

    // env(name) — read an environment variable. Returns nil if unset.
    let env_fn = lua.create_function(|lua, name: String| {
        match std::env::var(&name) {
            Ok(val) => Ok(LuaValue::String(lua.create_string(&val)?)),
            Err(_) => Ok(LuaValue::Nil),
        }
    })?;
    lua.globals().set("env", env_fn)?;

    // resolve_icon(name) — find the file path for an icon name.
    // Returns the path string or nil.
    let resolve_icon_fn = lua.create_function(|lua, name: String| {
        match pulpkit_render::resolve_icon_path(&name) {
            Some(path) => Ok(LuaValue::String(lua.create_string(path.to_string_lossy().as_ref())?)),
            None => Ok(LuaValue::Nil),
        }
    })?;
    lua.globals().set("resolve_icon", resolve_icon_fn)?;

    Ok(())
}
