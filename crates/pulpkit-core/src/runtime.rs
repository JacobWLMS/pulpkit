//! Main runtime — wires Lua, layout, rendering, and Wayland together.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use mlua::prelude::*;
use pulpkit_layout::{Theme, compute_layout, paint_tree};
use pulpkit_lua::{LuaNode, LuaVm, register_signal_api, register_widgets, register_window_fn};
use pulpkit_reactive::ReactiveRuntime;
use pulpkit_render::{Canvas, Color, TextRenderer};
use pulpkit_wayland::{Anchor, Layer, LayerSurface, OutputInfo, SurfaceConfig, WaylandClient};

use crate::ipc::IpcServer;

/// Run the shell defined in `shell_dir`.
///
/// This is the main entry point: it creates the reactive runtime, Lua VM,
/// loads the shell configuration, connects to Wayland, creates surfaces,
/// renders the initial frame, and enters the event loop.
pub fn run(shell_dir: std::path::PathBuf) -> anyhow::Result<()> {
    log::info!("Starting pulpkit with shell dir: {}", shell_dir.display());

    // Validate shell directory exists
    if !shell_dir.is_dir() {
        anyhow::bail!("Shell directory does not exist: {}", shell_dir.display());
    }

    // Create reactive runtime and enter its context.
    let rt = ReactiveRuntime::new();
    rt.enter(|| run_inner(&shell_dir))
}

/// Inner runtime logic, executed inside the reactive context.
fn run_inner(shell_dir: &Path) -> anyhow::Result<()> {
    let _ipc = IpcServer::new();

    // 1. Create the Lua VM.
    let vm = LuaVm::new().map_err(|e| anyhow::anyhow!("Failed to create Lua VM: {e}"))?;
    let lua = vm.lua();

    // 2. Load the theme from theme.lua (or use default if not present).
    let theme = load_theme(lua, shell_dir)?;
    let theme = Arc::new(theme);
    log::info!("Theme loaded (font: {})", theme.font_family);

    // 3. Inject widget constructors and signal API into the Lua VM.
    register_widgets(lua, theme.clone())
        .map_err(|e| anyhow::anyhow!("Failed to register widgets: {e}"))?;
    register_signal_api(lua)
        .map_err(|e| anyhow::anyhow!("Failed to register signal API: {e}"))?;

    // 4. Register the window() function — collects WindowDefs during shell.lua execution.
    let registry = pulpkit_lua::WindowRegistry::default();
    register_window_fn(lua, registry.clone())
        .map_err(|e| anyhow::anyhow!("Failed to register window fn: {e}"))?;

    // 5. Execute shell.lua — this calls window() to register window definitions.
    let shell_path = shell_dir.join("shell.lua");
    if !shell_path.exists() {
        anyhow::bail!("shell.lua not found in {}", shell_dir.display());
    }
    vm.load_file(&shell_path)
        .map_err(|e| anyhow::anyhow!("Failed to load shell.lua: {e}"))?;
    log::info!("shell.lua loaded successfully");

    let window_defs = registry.borrow();
    if window_defs.is_empty() {
        anyhow::bail!("No windows defined in shell.lua — call window() at least once");
    }
    log::info!("{} window(s) defined", window_defs.len());

    // 6. Connect to Wayland.
    let mut client = WaylandClient::connect()?;
    log::info!("Connected to Wayland display");

    // Do an initial roundtrip to discover outputs.
    client
        .event_loop
        .dispatch(Duration::from_millis(100), &mut client.state)?;

    log::info!("{} output(s) detected", client.state.outputs.len());

    // 7. Create text renderer for layout measurements.
    let text_renderer = TextRenderer::new();

    // 8. For each WindowDef, create layer surfaces and render the initial frame.
    struct ManagedSurface {
        surface: LayerSurface,
        root: pulpkit_layout::Node,
    }

    let mut surfaces: Vec<ManagedSurface> = Vec::new();

    for window_def in window_defs.iter() {
        log::info!(
            "Creating window '{}' (anchor={}, exclusive={})",
            window_def.name,
            window_def.anchor,
            window_def.exclusive
        );

        // Determine which outputs to create surfaces on.
        // Clone the OutputInfo to avoid borrowing client.state across mutable uses.
        let target_outputs: Vec<Option<OutputInfo>> = match &window_def.monitor {
            pulpkit_lua::MonitorTarget::All => {
                if client.state.outputs.is_empty() {
                    vec![None]
                } else {
                    client.state.outputs.iter().cloned().map(Some).collect()
                }
            }
            pulpkit_lua::MonitorTarget::Named(name) => {
                let found = client
                    .state
                    .outputs
                    .iter()
                    .find(|o| o.name == *name)
                    .cloned();
                vec![found]
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

            let ctx = lua
                .create_table()
                .map_err(|e| anyhow::anyhow!("Failed to create context table: {e}"))?;
            let monitor_table = lua
                .create_table()
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

            // Map anchor string to Anchor enum.
            let anchor = match window_def.anchor.as_str() {
                "top" => Anchor::Top,
                "bottom" => Anchor::Bottom,
                "left" => Anchor::Left,
                "right" => Anchor::Right,
                _ => Anchor::Top,
            };

            // Determine surface dimensions.
            let (width, height) = match anchor {
                Anchor::Top | Anchor::Bottom => {
                    let w = window_def.width.unwrap_or_else(|| {
                        maybe_output.as_ref().map(|o| o.width).unwrap_or(1920)
                    });
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
            };

            let mut surface = LayerSurface::new(&mut client.state, config)?;

            // Do a roundtrip to get the configure event from the compositor.
            client
                .event_loop
                .dispatch(Duration::from_millis(50), &mut client.state)?;

            // Handle any pending configures (resize the surface if needed).
            for configure in client.state.pending_configures.drain(..) {
                if configure.width > 0 && configure.height > 0 {
                    surface.resize(configure.width, configure.height);
                }
            }

            // Initial render.
            render_surface(&mut surface, &root_node, &text_renderer, &theme);

            surfaces.push(ManagedSurface {
                surface,
                root: root_node,
            });

            log::info!(
                "Surface created for '{}' ({}x{})",
                window_def.name,
                width,
                height
            );
        }
    }

    // Drop the borrow on window_defs before entering the event loop.
    drop(window_defs);

    // 9. Event loop — dispatch Wayland events.
    log::info!("Entering event loop");
    loop {
        client
            .event_loop
            .dispatch(Duration::from_millis(16), &mut client.state)?;

        // Handle configure events (resize).
        if !client.state.pending_configures.is_empty() {
            let configures: Vec<_> = client.state.pending_configures.drain(..).collect();
            for configure in configures {
                // For Plan 1, re-render all surfaces on any configure.
                for managed in &mut surfaces {
                    if configure.width > 0 && configure.height > 0 {
                        managed.surface.resize(configure.width, configure.height);
                    }
                    render_surface(
                        &mut managed.surface,
                        &managed.root,
                        &text_renderer,
                        &theme,
                    );
                }
            }
        }

        if client.state.exit_requested {
            log::info!("Exit requested by compositor");
            break;
        }
    }

    Ok(())
}

/// Render a widget tree onto a layer surface.
fn render_surface(
    surface: &mut LayerSurface,
    root: &pulpkit_layout::Node,
    text_renderer: &TextRenderer,
    theme: &Theme,
) {
    let width = surface.width();
    let height = surface.height();

    let layout =
        compute_layout(root, width as f32, height as f32, text_renderer, &theme.font_family);

    let buf = surface.get_buffer();
    let mut canvas = match Canvas::from_buffer(buf, width as i32, height as i32) {
        Some(c) => c,
        None => {
            log::error!("Failed to create Skia canvas ({}x{})", width, height);
            return;
        }
    };

    let bg_color = theme.colors.get("base").copied().unwrap_or_default();
    canvas.clear(bg_color);
    paint_tree(&mut canvas, &layout, &theme.font_family);
    canvas.flush();

    surface.commit();
}

/// Load a Theme from `theme.lua` in the shell directory.
///
/// If `theme.lua` does not exist, returns a default slate theme.
fn load_theme(lua: &Lua, shell_dir: &Path) -> anyhow::Result<Theme> {
    let theme_path = shell_dir.join("theme.lua");
    if !theme_path.exists() {
        log::info!("No theme.lua found, using default slate theme");
        return Ok(Theme::default_slate());
    }

    let code = std::fs::read_to_string(&theme_path)?;
    let theme_table: LuaTable = lua
        .load(&code)
        .set_name(theme_path.to_string_lossy())
        .eval()
        .map_err(|e| anyhow::anyhow!("Failed to evaluate theme.lua: {e}"))?;

    // Parse colors table.
    let mut colors = HashMap::new();
    if let Ok(colors_table) = theme_table.get::<LuaTable>("colors") {
        for pair in colors_table.pairs::<String, String>() {
            let (name, hex) = pair.map_err(|e| anyhow::anyhow!("Error reading colors: {e}"))?;
            if let Some(c) = Color::from_hex(&hex) {
                colors.insert(name, c);
            }
        }
    }

    // Parse spacing_scale.
    let spacing_scale: f32 = theme_table
        .get::<Option<f32>>("spacing_scale")
        .unwrap_or(None)
        .unwrap_or(4.0);

    // Parse rounding table.
    let mut rounding = HashMap::new();
    if let Ok(rounding_table) = theme_table.get::<LuaTable>("rounding") {
        for pair in rounding_table.pairs::<String, f32>() {
            let (name, val) =
                pair.map_err(|e| anyhow::anyhow!("Error reading rounding: {e}"))?;
            rounding.insert(name, val);
        }
    }

    // Parse font_sizes table.
    let mut font_sizes = HashMap::new();
    if let Ok(sizes_table) = theme_table.get::<LuaTable>("font_sizes") {
        for pair in sizes_table.pairs::<String, f32>() {
            let (name, val) =
                pair.map_err(|e| anyhow::anyhow!("Error reading font_sizes: {e}"))?;
            font_sizes.insert(name, val);
        }
    }

    // Parse font_family.
    let font_family: String = theme_table
        .get::<Option<String>>("font_family")
        .unwrap_or(None)
        .unwrap_or_else(|| "JetBrainsMono Nerd Font".into());

    // Fill in defaults for any missing sections.
    let default = Theme::default_slate();
    if colors.is_empty() {
        colors = default.colors;
    }
    if rounding.is_empty() {
        rounding = default.rounding;
    }
    if font_sizes.is_empty() {
        font_sizes = default.font_sizes;
    }

    Ok(Theme {
        colors,
        spacing_scale,
        rounding,
        font_sizes,
        font_family,
    })
}
