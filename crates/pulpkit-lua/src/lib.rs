//! Pulpkit Lua scripting — VM setup and widget constructor functions.

pub mod vm;
pub mod widgets;
pub mod window;

pub use vm::LuaVm;
pub use widgets::{LuaNode, register_widgets};
pub use window::{MonitorTarget, WindowDef, WindowRegistry, register_window_fn};

#[cfg(test)]
mod tests {
    use super::*;
    use pulpkit_layout::{Theme, tree::Node};
    use std::sync::Arc;

    #[test]
    fn lua_builds_node_tree() {
        let vm = LuaVm::new().unwrap();
        let theme = Arc::new(Theme::default_slate());
        register_widgets(vm.lua(), theme).unwrap();

        vm.lua()
            .load(
                r#"
            result = row("bg-base p-2 gap-4", {
                text("text-sm text-fg", "Hello"),
                spacer(),
                text("text-sm text-primary", "World"),
            })
        "#,
            )
            .exec()
            .unwrap();

        let result: mlua::AnyUserData = vm.lua().globals().get("result").unwrap();
        let node = result.borrow::<LuaNode>().unwrap();
        match &node.0 {
            Node::Container { children, .. } => assert_eq!(children.len(), 3),
            _ => panic!("expected Container"),
        }
    }

    #[test]
    fn lua_col_and_text() {
        let vm = LuaVm::new().unwrap();
        let theme = Arc::new(Theme::default_slate());
        register_widgets(vm.lua(), theme).unwrap();

        vm.lua()
            .load(
                r#"
            result = col("p-4", {
                text("text-lg font-bold", "Title"),
                text("text-sm text-muted", "Subtitle"),
            })
        "#,
            )
            .exec()
            .unwrap();

        let result: mlua::AnyUserData = vm.lua().globals().get("result").unwrap();
        let node = result.borrow::<LuaNode>().unwrap();
        match &node.0 {
            Node::Container {
                children,
                direction,
                ..
            } => {
                assert_eq!(children.len(), 2);
                assert!(matches!(direction, pulpkit_layout::Direction::Column));
            }
            _ => panic!("expected Container"),
        }
    }

    #[test]
    fn lua_window_registration() {
        let vm = LuaVm::new().unwrap();
        let theme = Arc::new(Theme::default_slate());
        register_widgets(vm.lua(), theme).unwrap();

        let registry: WindowRegistry = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        register_window_fn(vm.lua(), registry.clone()).unwrap();

        vm.lua()
            .load(
                r#"
            window("bar", {
                monitor = "all",
                anchor = "top",
                exclusive = true,
                height = 36,
            }, function(ctx)
                return row("w-full h-full bg-base px-2 items-center", {
                    text("text-sm text-primary font-bold", "Pulpkit"),
                    spacer(),
                    text("text-xs text-muted", "Hello World"),
                })
            end)
        "#,
            )
            .exec()
            .unwrap();

        let defs = registry.borrow();
        assert_eq!(defs.len(), 1);
        assert_eq!(defs[0].name, "bar");
        assert_eq!(defs[0].anchor, "top");
        assert!(defs[0].exclusive);
        assert_eq!(defs[0].height, Some(36));
        assert!(matches!(defs[0].monitor, MonitorTarget::All));
    }
}
