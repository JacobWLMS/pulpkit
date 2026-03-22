//! Pulpkit Lua scripting — VM setup and widget constructor functions.

pub mod signals;
pub mod vm;
pub mod widgets;
pub mod window;

pub use signals::{DynValue, LuaComputed, LuaSignal, register_signal_api};
pub use vm::LuaVm;
pub use widgets::{LuaNode, register_widgets};
pub use window::{MonitorTarget, WindowDef, WindowRegistry, register_window_fn};

#[cfg(test)]
mod tests {
    use super::*;
    use pulpkit_layout::{Theme, tree::Node};
    use pulpkit_reactive::ReactiveRuntime;
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

    #[test]
    fn lua_signal_get_set() {
        let rt = ReactiveRuntime::new();
        rt.enter(|| {
            let vm = LuaVm::new().unwrap();
            register_signal_api(vm.lua()).unwrap();

            vm.lua()
                .load(
                    r#"
                local count = signal(0)
                assert(count:get() == 0)
                count:set(42)
                assert(count:get() == 42)
            "#,
                )
                .exec()
                .unwrap();
        });
    }

    #[test]
    fn lua_computed_tracks_signal() {
        let rt = ReactiveRuntime::new();
        rt.enter(|| {
            let vm = LuaVm::new().unwrap();
            register_signal_api(vm.lua()).unwrap();

            vm.lua()
                .load(
                    r#"
                local count = signal(5)
                local doubled = computed(function()
                    return count:get() * 2
                end)
                assert(doubled:get() == 10)
                count:set(7)
                assert(doubled:get() == 14)
            "#,
                )
                .exec()
                .unwrap();
        });
    }

    #[test]
    fn lua_effect_runs_on_signal_change() {
        let rt = ReactiveRuntime::new();
        rt.enter(|| {
            let vm = LuaVm::new().unwrap();
            register_signal_api(vm.lua()).unwrap();

            // Use globals so they survive across load() chunks.
            vm.lua()
                .load(
                    r#"
                count = signal(0)
                observed = signal(-1)
                effect(function()
                    observed:set(count:get())
                end)
                assert(observed:get() == 0, "effect should run immediately")
                count:set(5)
            "#,
                )
                .exec()
                .unwrap();

            // Flush queued effects
            rt.flush();

            vm.lua()
                .load(
                    r#"
                assert(observed:get() == 5, "effect should have re-run after flush")
            "#,
                )
                .exec()
                .unwrap();
        });
    }

    #[test]
    fn lua_signal_string_value() {
        let rt = ReactiveRuntime::new();
        rt.enter(|| {
            let vm = LuaVm::new().unwrap();
            register_signal_api(vm.lua()).unwrap();

            vm.lua()
                .load(
                    r#"
                local name = signal("hello")
                assert(name:get() == "hello")
                name:set("world")
                assert(name:get() == "world")
            "#,
                )
                .exec()
                .unwrap();
        });
    }

    #[test]
    fn lua_signal_bool_value() {
        let rt = ReactiveRuntime::new();
        rt.enter(|| {
            let vm = LuaVm::new().unwrap();
            register_signal_api(vm.lua()).unwrap();

            vm.lua()
                .load(
                    r#"
                local flag = signal(false)
                assert(flag:get() == false)
                flag:set(true)
                assert(flag:get() == true)
            "#,
                )
                .exec()
                .unwrap();
        });
    }

    #[test]
    fn lua_button_with_click_handler() {
        let rt = ReactiveRuntime::new();
        rt.enter(|| {
            let vm = LuaVm::new().unwrap();
            let theme = Arc::new(Theme::default_slate());
            register_widgets(vm.lua(), theme).unwrap();
            register_signal_api(vm.lua()).unwrap();

            vm.lua()
                .load(
                    r#"
                local clicked = signal(false)
                result = button("bg-surface", {
                    on_click = function() clicked:set(true) end,
                }, {
                    text("text-sm text-fg", "Click me"),
                })
                -- Can't test click dispatch here, but verify structure
                assert(result ~= nil)
            "#,
                )
                .exec()
                .unwrap();

            let result: mlua::AnyUserData = vm.lua().globals().get("result").unwrap();
            let node = result.borrow::<LuaNode>().unwrap();
            match &node.0 {
                Node::Button {
                    children, handlers, ..
                } => {
                    assert_eq!(children.len(), 1);
                    assert!(handlers.on_click.is_some());
                }
                _ => panic!("expected Button"),
            }
        });
    }
}
