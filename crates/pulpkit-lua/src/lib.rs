//! Pulpkit Lua scripting — VM setup and widget constructor functions.

pub mod signals;
pub mod system;
pub mod timers;
pub mod vm;
pub mod widgets;
pub mod window;

pub use signals::{LuaComputed, LuaSignal, register_signal_api};
pub use pulpkit_reactive::DynValue;
pub use timers::{TimerDef, TimerRegistry, CancelledTimers, register_timer_api};
pub use system::{register_system_api, StreamDef, StreamRegistry};
pub use vm::LuaVm;
pub use widgets::{LuaNode, register_widgets};
pub use window::{MonitorTarget, PopupDef, PopupRegistry, WindowDef, WindowRegistry, register_popup_fn, register_window_fn};

#[cfg(test)]
mod tests {
    use super::*;
    use pulpkit_layout::{Theme, tree::{Node, InteractiveKind}};
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
                Node::Interactive {
                    kind: InteractiveKind::Button { handlers },
                    children,
                    ..
                } => {
                    assert_eq!(children.len(), 1);
                    assert!(handlers.on_click.is_some());
                }
                _ => panic!("expected Interactive/Button"),
            }
        });
    }

    #[test]
    fn lua_popup_registration() {
        let rt = ReactiveRuntime::new();
        rt.enter(|| {
            let vm = LuaVm::new().unwrap();
            let theme = Arc::new(Theme::default_slate());
            register_widgets(vm.lua(), theme).unwrap();
            register_signal_api(vm.lua()).unwrap();

            let popup_reg: PopupRegistry = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
            register_popup_fn(vm.lua(), popup_reg.clone()).unwrap();

            vm.lua()
                .load(
                    r#"
                local show = signal(false)
                popup("test-popup", {
                    parent = "bar",
                    anchor = "top right",
                    offset = { x = -8, y = 4 },
                    dismiss_on_outside = true,
                    visible = show,
                    width = 280,
                    height = 200,
                }, function()
                    return col("bg-surface rounded-lg p-4 w-full h-full", {
                        text("text-sm text-fg", "Popup content"),
                    })
                end)
            "#,
                )
                .exec()
                .unwrap();

            let defs = popup_reg.borrow();
            assert_eq!(defs.len(), 1);
            assert_eq!(defs[0].name, "test-popup");
            assert_eq!(defs[0].parent, "bar");
            assert_eq!(defs[0].anchor, "top right");
            assert_eq!(defs[0].offset, (-8, 4));
            assert!(defs[0].dismiss_on_outside);
            assert!(defs[0].visible_signal.is_some());
            assert_eq!(defs[0].width, Some(280));
            assert_eq!(defs[0].height, Some(200));
        });
    }

    #[test]
    fn lua_slider_creation() {
        let rt = ReactiveRuntime::new();
        rt.enter(|| {
            let vm = LuaVm::new().unwrap();
            let theme = Arc::new(Theme::default_slate());
            register_widgets(vm.lua(), theme).unwrap();
            register_signal_api(vm.lua()).unwrap();

            vm.lua()
                .load(
                    r#"
                local vol = signal(75)
                result = slider("w-full", {
                    value = vol,
                    on_change = function(v) vol:set(v) end,
                    min = 0,
                    max = 100,
                })
            "#,
                )
                .exec()
                .unwrap();

            let result: mlua::AnyUserData = vm.lua().globals().get("result").unwrap();
            let node = result.borrow::<LuaNode>().unwrap();
            match &node.0 {
                Node::Interactive {
                    kind: InteractiveKind::Slider { value, min, max, on_change, .. },
                    ..
                } => {
                    assert!((*min - 0.0).abs() < f64::EPSILON);
                    assert!((*max - 100.0).abs() < f64::EPSILON);
                    assert!(on_change.is_some());
                    // Value signal should hold the initial value from Lua.
                    assert!((value.get().as_f64() - 75.0).abs() < f64::EPSILON);
                }
                _ => panic!("expected Interactive/Slider"),
            }
        });
    }

    #[test]
    fn lua_popup_visible_signal_controls_visibility() {
        let rt = ReactiveRuntime::new();
        rt.enter(|| {
            let vm = LuaVm::new().unwrap();
            let theme = Arc::new(Theme::default_slate());
            register_widgets(vm.lua(), theme).unwrap();
            register_signal_api(vm.lua()).unwrap();

            let popup_reg: PopupRegistry = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
            register_popup_fn(vm.lua(), popup_reg.clone()).unwrap();

            vm.lua()
                .load(
                    r#"
                show_popup = signal(false)
                popup("vis-test", {
                    parent = "bar",
                    anchor = "top left",
                    visible = show_popup,
                    width = 100,
                    height = 100,
                }, function()
                    return text("text-sm", "Hello")
                end)
            "#,
                )
                .exec()
                .unwrap();

            let defs = popup_reg.borrow();
            let sig = defs[0].visible_signal.as_ref().unwrap();

            // Initially false
            assert_eq!(sig.get(), DynValue::Bool(false));

            // Set to true from Lua
            vm.lua()
                .load(r#"show_popup:set(true)"#)
                .exec()
                .unwrap();
            assert_eq!(sig.get(), DynValue::Bool(true));

            // Set back to false
            vm.lua()
                .load(r#"show_popup:set(false)"#)
                .exec()
                .unwrap();
            assert_eq!(sig.get(), DynValue::Bool(false));
        });
    }

    #[test]
    fn lua_toggle_creation() {
        let rt = ReactiveRuntime::new();
        rt.enter(|| {
            let vm = LuaVm::new().unwrap();
            let theme = Arc::new(Theme::default_slate());
            register_widgets(vm.lua(), theme).unwrap();
            register_signal_api(vm.lua()).unwrap();

            vm.lua()
                .load(
                    r#"
                local muted = signal(false)
                result = toggle("", {
                    checked = muted,
                    on_change = function(v) muted:set(v) end,
                })
            "#,
                )
                .exec()
                .unwrap();

            let result: mlua::AnyUserData = vm.lua().globals().get("result").unwrap();
            let node = result.borrow::<LuaNode>().unwrap();
            match &node.0 {
                Node::Interactive {
                    kind: InteractiveKind::Toggle { checked, on_change, .. },
                    ..
                } => {
                    assert_eq!(checked.get().as_bool(), false);
                    assert!(on_change.is_some());
                }
                _ => panic!("expected Interactive/Toggle"),
            }
        });
    }
}
