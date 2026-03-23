//! Pulpkit Lua scripting — Elm bridge, widget constructors, msg API.

pub mod bridge;
pub mod element;
pub mod msg;
pub mod subscribe;
pub mod vm;
pub mod widgets;

pub use bridge::ElmBridge;
pub use element::{LuaElement, lua_to_element};
pub use msg::{register_msg_api, is_msg_table, lua_table_to_message, lua_value_to_message_data, message_to_lua_table};
pub use subscribe::{SubscriptionDef, register_subscribe_api, parse_subscriptions};
pub use vm::LuaVm;
pub use widgets::{register_widgets, lua_table_to_surface_def};
