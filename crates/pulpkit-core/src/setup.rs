//! Shell setup — creates surfaces, popups, and intervals from Lua definitions.

use std::cell::Cell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use mlua::prelude::*;
use pulpkit_layout::Theme;
use pulpkit_lua::LuaNode;
use pulpkit_render::TextRenderer;
use pulpkit_wayland::{
    Anchor, Layer, LayerSurface, OutputInfo, PopupAnchor, SurfaceConfig, SurfaceMargins,
    WaylandClient,
};

use crate::popups::{ManagedPopup, PopupConfig, PopupState};
use crate::surfaces::ManagedSurface;
use crate::timers::ActiveTimer;

pub fn create_surfaces(
    window_defs: &[pulpkit_lua::WindowDef],
    lua: &Lua,
    client: &mut WaylandClient,
    text_renderer: &TextRenderer,
    theme: &Theme,
) -> anyhow::Result<Vec<ManagedSurface>> {
    let mut surfaces = Vec::new();

    for window_def in window_defs {
        log::info!(
            "Creating window '{}' (anchor={}, exclusive={})",
            window_def.name, window_def.anchor, window_def.exclusive,
        );

        let target_outputs = resolve_outputs(&window_def.monitor, &client.state.outputs);

        for maybe_output in &target_outputs {
            let root_node = call_widget_fn(lua, &window_def.widget_fn, maybe_output)?;
            let anchor = parse_anchor(&window_def.anchor);
            let (width, height) = compute_dimensions(window_def, anchor, maybe_output);

            let exclusive_zone = if window_def.exclusive {
                match anchor {
                    Anchor::Top | Anchor::Bottom => height as i32,
                    Anchor::Left | Anchor::Right => width as i32,
                }
            } else {
                -1
            };

            let config = SurfaceConfig {
                width,
                height,
                anchor,
                layer: Layer::Top,
                exclusive_zone,
                namespace: window_def.namespace.clone(),
                output: maybe_output.as_ref().map(|o| o.wl_output.clone()),
                margins: SurfaceMargins::default(),
            };

            let mut surface = LayerSurface::new(&mut client.state, config)?;

            client
                .event_loop
                .dispatch(Duration::from_millis(50), &mut client.state)?;

            for configure in client.state.pending_configures.drain(..) {
                if configure.width > 0
                    && configure.height > 0
                    && configure.surface_id == surface.surface_id()
                {
                    surface.resize(configure.width, configure.height);
                }
            }

            let mut managed = ManagedSurface {
                name: window_def.name.clone(),
                surface,
                root: root_node,
                layout: None,
                dirty: Rc::new(Cell::new(true)),
                hovered_node: None,
            };
            managed.render(text_renderer, theme);

            log::info!("Surface created for '{}' ({}x{})", window_def.name, width, height);
            surfaces.push(managed);
        }
    }

    Ok(surfaces)
}

pub fn create_popups(
    popup_defs: &[pulpkit_lua::PopupDef],
    lua: &Lua,
    client: &WaylandClient,
) -> anyhow::Result<Vec<ManagedPopup>> {
    let mut popups = Vec::new();

    for popup_def in popup_defs {
        log::info!(
            "Registering popup '{}' (parent={}, anchor={})",
            popup_def.name, popup_def.parent, popup_def.anchor,
        );

        let widget_fn: LuaFunction = lua
            .registry_value(&popup_def.widget_fn_key)
            .map_err(|e| anyhow::anyhow!("Failed to get popup widget function: {e}"))?;
        let result: LuaAnyUserData = widget_fn
            .call(())
            .map_err(|e| anyhow::anyhow!("Popup widget function failed: {e}"))?;
        let lua_node = result
            .borrow::<LuaNode>()
            .map_err(|e| anyhow::anyhow!("Popup widget fn did not return a LuaNode: {e}"))?;

        let popup_anchor = match popup_def.anchor.as_str() {
            "top right" => PopupAnchor::TopRight,
            "bottom left" => PopupAnchor::BottomLeft,
            "bottom right" => PopupAnchor::BottomRight,
            _ => PopupAnchor::TopLeft,
        };

        popups.push(ManagedPopup {
            name: popup_def.name.clone(),
            root: lua_node.0.clone(),
            state: PopupState::Hidden,
            config: PopupConfig {
                parent_name: popup_def.parent.clone(),
                anchor: popup_anchor,
                offset: popup_def.offset,
                dismiss_on_outside: popup_def.dismiss_on_outside,
                width: popup_def.width.unwrap_or(280),
                height: popup_def.height.unwrap_or(200),
                output: client.state.outputs.first().cloned(),
            },
            visible_signal: popup_def.visible_signal.clone(),
        });

        log::info!("Popup '{}' registered (starts hidden)", popup_def.name);
    }

    Ok(popups)
}

pub fn create_timers(
    timer_defs: &[pulpkit_lua::TimerDef],
    lua: &Lua,
) -> anyhow::Result<Vec<ActiveTimer>> {
    let now = Instant::now();
    let mut timers = Vec::new();

    for def in timer_defs {
        let interval = Duration::from_millis(def.interval_ms);
        let callback_key = lua
            .registry_value::<LuaFunction>(&def.callback_key)
            .and_then(|f| lua.create_registry_value(f))
            .map_err(|e| anyhow::anyhow!("Failed to clone timer callback: {e}"))?;

        timers.push(ActiveTimer {
            id: def.id,
            callback_key,
            interval,
            next_fire: now + interval,
            one_shot: def.one_shot,
            cancelled: false,
        });
    }

    if !timers.is_empty() {
        log::info!("{} timer(s) registered", timers.len());
    }

    Ok(timers)
}

fn resolve_outputs(
    monitor: &pulpkit_lua::MonitorTarget,
    outputs: &[OutputInfo],
) -> Vec<Option<OutputInfo>> {
    match monitor {
        pulpkit_lua::MonitorTarget::All => {
            if outputs.is_empty() { vec![None] }
            else { outputs.iter().cloned().map(Some).collect() }
        }
        pulpkit_lua::MonitorTarget::Named(name) => {
            vec![outputs.iter().find(|o| o.name == *name).cloned()]
        }
        pulpkit_lua::MonitorTarget::Focused => {
            if outputs.is_empty() { vec![None] }
            else { vec![Some(outputs[0].clone())] }
        }
    }
}

fn call_widget_fn(
    lua: &Lua,
    widget_fn_key: &mlua::RegistryKey,
    maybe_output: &Option<OutputInfo>,
) -> anyhow::Result<pulpkit_layout::Node> {
    let widget_fn: LuaFunction = lua
        .registry_value(widget_fn_key)
        .map_err(|e| anyhow::anyhow!("Failed to get widget function: {e}"))?;

    let ctx = lua.create_table()
        .map_err(|e| anyhow::anyhow!("Failed to create context table: {e}"))?;
    let mt = lua.create_table()
        .map_err(|e| anyhow::anyhow!("Failed to create monitor table: {e}"))?;

    if let Some(output) = maybe_output {
        mt.set("name", output.name.clone()).ok();
        mt.set("width", output.width).ok();
        mt.set("height", output.height).ok();
    } else {
        mt.set("name", "unknown").ok();
        mt.set("width", 1920u32).ok();
        mt.set("height", 1080u32).ok();
    }
    ctx.set("monitor", mt).ok();

    let result: LuaAnyUserData = widget_fn
        .call(ctx)
        .map_err(|e| anyhow::anyhow!("Widget function failed: {e}"))?;
    let lua_node = result
        .borrow::<LuaNode>()
        .map_err(|e| anyhow::anyhow!("Widget fn did not return a LuaNode: {e}"))?;
    Ok(lua_node.0.clone())
}

fn parse_anchor(s: &str) -> Anchor {
    match s {
        "top" => Anchor::Top,
        "bottom" => Anchor::Bottom,
        "left" => Anchor::Left,
        "right" => Anchor::Right,
        _ => Anchor::Top,
    }
}

fn compute_dimensions(
    def: &pulpkit_lua::WindowDef,
    anchor: Anchor,
    output: &Option<OutputInfo>,
) -> (u32, u32) {
    match anchor {
        Anchor::Top | Anchor::Bottom => {
            let w = def.width.unwrap_or_else(|| output.as_ref().map(|o| o.width).unwrap_or(1920));
            (w, def.height.unwrap_or(36))
        }
        Anchor::Left | Anchor::Right => {
            let h = def.height.unwrap_or_else(|| output.as_ref().map(|o| o.height).unwrap_or(1080));
            (def.width.unwrap_or(48), h)
        }
    }
}
