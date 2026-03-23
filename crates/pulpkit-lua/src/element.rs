//! LuaElement userdata — wraps Element for passing between Lua and Rust.

use mlua::prelude::*;
use pulpkit_layout::element::Element;

/// Wrapper around Element for Lua userdata.
#[derive(Debug, Clone)]
pub struct LuaElement(pub Element);

impl LuaUserData for LuaElement {}

/// Extract an Element from a Lua value (LuaElement userdata).
pub fn lua_to_element(val: &LuaValue) -> LuaResult<Element> {
    match val {
        LuaValue::UserData(ud) => {
            let el = ud.borrow::<LuaElement>()?;
            Ok(el.0.clone())
        }
        _ => Err(LuaError::RuntimeError(format!(
            "expected widget element, got {:?}",
            val.type_name()
        ))),
    }
}

/// Extract a list of Elements from Lua varargs.
pub fn collect_children(args: &[LuaValue]) -> LuaResult<Vec<Element>> {
    let mut children = Vec::new();
    for val in args {
        match val {
            LuaValue::UserData(ud) => {
                let el = ud.borrow::<LuaElement>()?;
                children.push(el.0.clone());
            }
            // Skip nil values (trailing nils from varargs)
            LuaValue::Nil => {}
            _ => {
                return Err(LuaError::RuntimeError(format!(
                    "expected widget element as child, got {:?}",
                    val.type_name()
                )));
            }
        }
    }
    Ok(children)
}
