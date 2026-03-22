//! Runtime orchestration — setup, Lua loading, surface creation, event loop entry.

use std::cell::Cell;
use std::path::Path;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use calloop::channel;
use mlua::prelude::*;
use pulpkit_lua::{
    LuaNode, LuaVm,
    register_interval_fn, register_popup_fn, register_signal_api, register_widgets,
    register_window_fn,
};
use pulpkit_reactive::ReactiveRuntime;
use pulpkit_layout::Theme;
use pulpkit_render::TextRenderer;
use pulpkit_wayland::{
    Anchor, Layer, LayerSurface, OutputInfo, PopupAnchor, SurfaceConfig, SurfaceMargins,
    WaylandClient,
};

use crate::event_loop;
use crate::ipc::IpcServer;
use crate::popups::{ManagedPopup, PopupConfig, PopupState};
use crate::surfaces::ManagedSurface;
use crate::theme::load_theme;
use crate::timers::ActiveInterval;
use crate::watcher;

/// Internal events that can wake the event loop from its idle sleep.
#[allow(dead_code)]
pub enum RuntimeEvent {
    Redraw,
}

/// Run the shell defined in `shell_dir`.
pub fn run(shell_dir: std::path::PathBuf) -> anyhow::Result<()> {
    log::info!("Starting pulpkit with shell dir: {}", shell_dir.display());

    if !shell_dir.is_dir() {
        anyhow::bail!("Shell directory does not exist: {}", shell_dir.display());
    }

    let rt = ReactiveRuntime::new();
    rt.enter(|| run_inner(&shell_dir, &rt))
}

/// Inner runtime logic, executed inside the reactive context.
fn run_inner(shell_dir: &Path, rt: &ReactiveRuntime) -> anyhow::Result<()> {
    let _ipc = IpcServer::new();

    // 0. File watcher (hot-reload — Wave 2).
    let watcher = watcher::FileWatcher::new(shell_dir)?;
    log::info!("File watcher active on {}", shell_dir.display());
    while watcher.poll().is_some() {} // drain startup noise

    // 1. Lua VM.
    let vm = LuaVm::new().map_err(|e| anyhow::anyhow!("Failed to create Lua VM: {e}"))?;
    let lua = vm.lua();

    // 2. Theme.
    let theme = Arc::new(load_theme(lua, shell_dir)?);
    log::info!("Theme loaded (font: {})", theme.font_family);

    // 3. Register Lua APIs.
    register_widgets(lua, theme.clone())
        .map_err(|e| anyhow::anyhow!("Failed to register widgets: {e}"))?;
    register_signal_api(lua)
        .map_err(|e| anyhow::anyhow!("Failed to register signal API: {e}"))?;

    let window_registry = pulpkit_lua::WindowRegistry::default();
    register_window_fn(lua, window_registry.clone())
        .map_err(|e| anyhow::anyhow!("Failed to register window fn: {e}"))?;

    let popup_registry = pulpkit_lua::PopupRegistry::default();
    register_popup_fn(lua, popup_registry.clone())
        .map_err(|e| anyhow::anyhow!("Failed to register popup fn: {e}"))?;

    let interval_registry = pulpkit_lua::IntervalRegistry::default();
    register_interval_fn(lua, interval_registry.clone())
        .map_err(|e| anyhow::anyhow!("Failed to register set_interval fn: {e}"))?;

    // 4. Execute shell.lua.
    let shell_path = shell_dir.join("shell.lua");
    if !shell_path.exists() {
        anyhow::bail!("shell.lua not found in {}", shell_dir.display());
    }
    vm.load_file(&shell_path)
        .map_err(|e| anyhow::anyhow!("Failed to load shell.lua: {e}"))?;
    log::info!("shell.lua loaded successfully");

    let window_defs = window_registry.borrow();
    if window_defs.is_empty() {
        anyhow::bail!("No windows defined in shell.lua — call window() at least once");
    }
    log::info!("{} window(s) defined", window_defs.len());

    // 5. Connect to Wayland.
    let mut client = WaylandClient::connect()?;
    log::info!("Connected to Wayland display");

    // Insert calloop wake channel.
    let (wake_sender, wake_channel) = channel::channel::<RuntimeEvent>();
    client
        .event_loop
        .handle()
        .insert_source(wake_channel, |event, _, _state| {
            if let channel::Event::Msg(msg) = event {
                match msg {
                    RuntimeEvent::Redraw => log::debug!("Wake: Redraw requested"),
                }
            }
        })
        .map_err(|e| anyhow::anyhow!("Failed to insert wake channel: {e}"))?;
    // Initial roundtrip to discover outputs.
    client
        .event_loop
        .dispatch(Duration::from_millis(100), &mut client.state)?;
    log::info!("{} output(s) detected", client.state.outputs.len());

    // 6. Create text renderer.
    let text_renderer = TextRenderer::new();

    // 7. Create surfaces for each WindowDef.
    let mut surfaces = create_surfaces(
        &window_defs, lua, &mut client, &text_renderer, &theme,
    )?;
    drop(window_defs);

    // Wire dirty-tracking Effects for reactive Props.
    crate::dirty::wire_dirty_tracking(&surfaces, &wake_sender);

    // 8. Create popups for each PopupDef.
    let mut popups = create_popups(
        &popup_registry.borrow(), lua, &client,
    )?;

    // 9. Set up intervals.
    let mut intervals = create_intervals(&interval_registry.borrow(), lua)?;

    // 10. Enter the event loop.
    event_loop::run(
        &mut client,
        &mut surfaces,
        &mut popups,
        &mut intervals,
        lua,
        &text_renderer,
        &theme,
        rt,
    )
}

// ---------------------------------------------------------------------------
// Setup helpers
// ---------------------------------------------------------------------------

fn create_surfaces(
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

        let target_outputs: Vec<Option<OutputInfo>> = match &window_def.monitor {
            pulpkit_lua::MonitorTarget::All => {
                if client.state.outputs.is_empty() {
                    vec![None]
                } else {
                    client.state.outputs.iter().cloned().map(Some).collect()
                }
            }
            pulpkit_lua::MonitorTarget::Named(name) => {
                vec![client.state.outputs.iter().find(|o| o.name == *name).cloned()]
            }
            pulpkit_lua::MonitorTarget::Focused => {
                if client.state.outputs.is_empty() {
                    vec![None]
                } else {
                    vec![Some(client.state.outputs[0].clone())]
                }
            }
        };

        for maybe_output in &target_outputs {
            // Call the widget function to get the root Node.
            let widget_fn: LuaFunction = lua
                .registry_value(&window_def.widget_fn)
                .map_err(|e| anyhow::anyhow!("Failed to get widget function: {e}"))?;

            let ctx = lua.create_table()
                .map_err(|e| anyhow::anyhow!("Failed to create context table: {e}"))?;
            let monitor_table = lua.create_table()
                .map_err(|e| anyhow::anyhow!("Failed to create monitor table: {e}"))?;
            if let Some(output) = maybe_output {
                monitor_table.set("name", output.name.clone()).ok();
                monitor_table.set("width", output.width).ok();
                monitor_table.set("height", output.height).ok();
            } else {
                monitor_table.set("name", "unknown").ok();
                monitor_table.set("width", 1920u32).ok();
                monitor_table.set("height", 1080u32).ok();
            }
            ctx.set("monitor", monitor_table).ok();

            let result: LuaAnyUserData = widget_fn
                .call(ctx)
                .map_err(|e| anyhow::anyhow!("Widget function failed: {e}"))?;
            let lua_node = result
                .borrow::<LuaNode>()
                .map_err(|e| anyhow::anyhow!("Widget function did not return a LuaNode: {e}"))?;
            let root_node = lua_node.0.clone();

            let anchor = match window_def.anchor.as_str() {
                "top" => Anchor::Top,
                "bottom" => Anchor::Bottom,
                "left" => Anchor::Left,
                "right" => Anchor::Right,
                _ => Anchor::Top,
            };

            let (width, height) = match anchor {
                Anchor::Top | Anchor::Bottom => {
                    let w = window_def
                        .width
                        .unwrap_or_else(|| maybe_output.as_ref().map(|o| o.width).unwrap_or(1920));
                    let h = window_def.height.unwrap_or(36);
                    (w, h)
                }
                Anchor::Left | Anchor::Right => {
                    let w = window_def.width.unwrap_or(48);
                    let h = window_def.height.unwrap_or_else(|| {
                        maybe_output.as_ref().map(|o| o.height).unwrap_or(1080)
                    });
                    (w, h)
                }
            };

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

            // Roundtrip for configure.
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

fn create_popups(
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
            .map_err(|e| anyhow::anyhow!("Popup widget function did not return a LuaNode: {e}"))?;
        let root_node = lua_node.0.clone();

        let popup_anchor = match popup_def.anchor.as_str() {
            "top right" => PopupAnchor::TopRight,
            "bottom left" => PopupAnchor::BottomLeft,
            "bottom right" => PopupAnchor::BottomRight,
            _ => PopupAnchor::TopLeft,
        };

        let output = client.state.outputs.first().cloned();

        popups.push(ManagedPopup {
            name: popup_def.name.clone(),
            root: root_node,
            state: PopupState::Hidden,
            config: PopupConfig {
                parent_name: popup_def.parent.clone(),
                anchor: popup_anchor,
                offset: popup_def.offset,
                dismiss_on_outside: popup_def.dismiss_on_outside,
                width: popup_def.width.unwrap_or(280),
                height: popup_def.height.unwrap_or(200),
                output,
            },
            visible_signal: popup_def.visible_signal.clone(),
        });

        log::info!("Popup '{}' registered (starts hidden)", popup_def.name);
    }

    Ok(popups)
}

fn create_intervals(
    interval_defs: &[pulpkit_lua::IntervalDef],
    lua: &Lua,
) -> anyhow::Result<Vec<ActiveInterval>> {
    let now = Instant::now();
    let mut intervals = Vec::new();

    for def in interval_defs {
        let interval = Duration::from_millis(def.interval_ms);
        let callback_key = lua
            .registry_value::<LuaFunction>(&def.callback_key)
            .and_then(|f| lua.create_registry_value(f))
            .map_err(|e| anyhow::anyhow!("Failed to clone interval callback: {e}"))?;

        intervals.push(ActiveInterval {
            callback_key,
            interval,
            next_fire: now + interval,
        });
    }

    if !intervals.is_empty() {
        log::info!("{} interval(s) registered", intervals.len());
    }

    Ok(intervals)
}

