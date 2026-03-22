//! Reactive signal/computed/effect bindings for Lua.
//!
//! Exposes `signal()`, `computed()`, and `effect()` as Lua globals so that
//! Lua scripts can create and interact with the reactive graph.

use std::rc::Rc;

use mlua::prelude::*;
use pulpkit_reactive::{Computed, Effect, Signal};

// ---------------------------------------------------------------------------
// DynValue — a Lua-compatible value that is Clone + 'static
// ---------------------------------------------------------------------------

/// A dynamic value that can cross the Rust ↔ Lua boundary and live inside a
/// reactive `Signal<DynValue>`.
#[derive(Debug, Clone, PartialEq)]
pub enum DynValue {
    Nil,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
}

/// Convert a `LuaValue` to a `DynValue`.
fn lua_to_dynvalue(value: &LuaValue) -> DynValue {
    match value {
        LuaValue::Nil => DynValue::Nil,
        LuaValue::Boolean(b) => DynValue::Bool(*b),
        LuaValue::Integer(i) => DynValue::Int(*i),
        LuaValue::Number(n) => DynValue::Float(*n),
        LuaValue::String(s) => DynValue::Str(s.to_string_lossy()),
        _ => DynValue::Nil,
    }
}

/// Convert a `DynValue` back to a `LuaValue`.
fn dynvalue_to_lua(lua: &Lua, val: &DynValue) -> LuaResult<LuaValue> {
    match val {
        DynValue::Nil => Ok(LuaValue::Nil),
        DynValue::Bool(b) => Ok(LuaValue::Boolean(*b)),
        DynValue::Int(i) => Ok(LuaValue::Integer(*i)),
        DynValue::Float(f) => Ok(LuaValue::Number(*f)),
        DynValue::Str(s) => Ok(LuaValue::String(lua.create_string(s)?)),
    }
}

// ---------------------------------------------------------------------------
// LuaSignal — wraps Signal<DynValue>
// ---------------------------------------------------------------------------

/// Lua userdata wrapping a reactive `Signal<DynValue>`.
#[derive(Clone)]
pub struct LuaSignal(pub Signal<DynValue>);

impl LuaUserData for LuaSignal {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("get", |lua, this, ()| {
            dynvalue_to_lua(lua, &this.0.get())
        });
        methods.add_method("set", |_lua, this, value: LuaValue| {
            this.0.set(lua_to_dynvalue(&value));
            Ok(())
        });
    }
}

// ---------------------------------------------------------------------------
// LuaComputed — wraps Computed<DynValue>
// ---------------------------------------------------------------------------

/// Lua userdata wrapping a reactive `Computed<DynValue>`.
#[derive(Clone)]
pub struct LuaComputed(pub Computed<DynValue>);

impl LuaUserData for LuaComputed {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("get", |lua, this, ()| {
            dynvalue_to_lua(lua, &this.0.get())
        });
    }
}

// ---------------------------------------------------------------------------
// Registration
// ---------------------------------------------------------------------------

/// Register `signal()`, `computed()`, and `effect()` as Lua globals.
pub fn register_signal_api(lua: &Lua) -> LuaResult<()> {
    // signal(initial_value) -> LuaSignal
    let signal_fn = lua.create_function(|_lua, value: LuaValue| {
        let sig = Signal::new(lua_to_dynvalue(&value));
        Ok(LuaSignal(sig))
    })?;
    lua.globals().set("signal", signal_fn)?;

    // computed(fn) -> LuaComputed
    {
        let lua_clone = lua.clone();
        let computed_fn = lua.create_function(move |lua, func: LuaFunction| {
            let key = Rc::new(lua.create_registry_value(func)?);
            let lua_handle = lua_clone.clone();
            let computed = Computed::new(move || {
                let func: LuaFunction = lua_handle
                    .registry_value(&key)
                    .expect("computed: registry lookup failed");
                let result: LuaValue = func.call(()).expect("computed: lua function call failed");
                lua_to_dynvalue(&result)
            });
            Ok(LuaComputed(computed))
        })?;
        lua.globals().set("computed", computed_fn)?;
    }

    // effect(fn) -> nil
    {
        let lua_clone = lua.clone();
        let effect_fn = lua.create_function(move |lua, func: LuaFunction| {
            let key = Rc::new(lua.create_registry_value(func)?);
            let lua_handle = lua_clone.clone();
            // Effect runs immediately and re-runs on flush() when deps change.
            // We keep the Effect alive by leaking it (it lives for the lifetime
            // of the reactive runtime). This is intentional — effects are
            // long-lived side-effects.
            let _effect = Effect::new(move || {
                let func: LuaFunction = lua_handle
                    .registry_value(&key)
                    .expect("effect: registry lookup failed");
                func.call::<()>(()).expect("effect: lua function call failed");
            });
            std::mem::forget(_effect);
            Ok(())
        })?;
        lua.globals().set("effect", effect_fn)?;
    }

    Ok(())
}
