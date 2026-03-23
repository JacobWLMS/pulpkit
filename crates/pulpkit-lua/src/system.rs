//! System interaction — exec, exec_output, exec_stream, env, resolve_icon.

use mlua::prelude::*;
use std::cell::RefCell;
use std::process::Command;
use std::rc::Rc;

/// A stream definition registered from Lua's exec_stream().
pub struct StreamDef {
    pub id: u64,
    pub cmd: String,
    pub callback_key: mlua::RegistryKey,
}

/// Shared registry for stream definitions.
pub type StreamRegistry = Rc<RefCell<Vec<StreamDef>>>;

/// Thread-local stream ID counter.
fn next_stream_id() -> u64 {
    thread_local! {
        static COUNTER: std::cell::Cell<u64> = const { std::cell::Cell::new(1) };
    }
    COUNTER.with(|c| {
        let id = c.get();
        c.set(id + 1);
        id
    })
}

/// Register system interaction functions as Lua globals.
pub fn register_system_api(
    lua: &Lua,
    stream_registry: StreamRegistry,
) -> LuaResult<()> {
    // exec(cmd) — fire-and-forget async command.
    let exec_fn = lua.create_function(|_lua, cmd: String| {
        std::thread::spawn(move || {
            let _ = Command::new("sh").arg("-c").arg(&cmd).output();
        });
        Ok(())
    })?;
    lua.globals().set("exec", exec_fn)?;

    // exec_output(cmd) — blocking command, returns stdout.
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

    // exec_stream(cmd, callback) — persistent subprocess with line-by-line output.
    // The callback is called for each line of stdout.
    // Returns a stream ID for cancellation.
    {
        let reg = stream_registry.clone();
        let stream_fn = lua.create_function(move |lua, (cmd, callback): (String, LuaFunction)| {
            let id = next_stream_id();
            let key = lua.create_registry_value(callback)?;
            reg.borrow_mut().push(StreamDef {
                id,
                cmd,
                callback_key: key,
            });
            Ok(id)
        })?;
        lua.globals().set("exec_stream", stream_fn)?;
    }

    // cancel_stream(id) — cancel a running stream.
    // (Cancellation is handled by the runtime via a shared cancelled set.)
    // For now this is a stub — cancellation will be wired in the event loop.
    let cancel_fn = lua.create_function(|_lua, _id: u64| {
        // TODO: wire cancellation
        Ok(())
    })?;
    lua.globals().set("cancel_stream", cancel_fn)?;

    // env(name) — read an environment variable.
    let env_fn = lua.create_function(|lua, name: String| {
        match std::env::var(&name) {
            Ok(val) => Ok(LuaValue::String(lua.create_string(&val)?)),
            Err(_) => Ok(LuaValue::Nil),
        }
    })?;
    lua.globals().set("env", env_fn)?;

    // resolve_icon(name) — find the file path for an icon name.
    let resolve_icon_fn = lua.create_function(|lua, name: String| {
        match pulpkit_render::resolve_icon_path(&name) {
            Some(path) => Ok(LuaValue::String(lua.create_string(path.to_string_lossy().as_ref())?)),
            None => Ok(LuaValue::Nil),
        }
    })?;
    lua.globals().set("resolve_icon", resolve_icon_fn)?;

    Ok(())
}
