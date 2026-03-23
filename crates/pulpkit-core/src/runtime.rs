//! Runtime orchestration — setup, Lua loading, surface creation, event loop entry.

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use calloop::channel;
use pulpkit_lua::{
    ElmBridge, LuaVm, register_msg_api, register_subscribe_api, register_widgets,
};
use pulpkit_layout::element::{Message, SurfaceDef, SurfaceKind, MonitorTarget};
use pulpkit_layout::Theme;
use pulpkit_render::TextRenderer;
use pulpkit_sub::{SubMessage, SubscriptionManager};
use pulpkit_wayland::{Anchor, Layer, SurfaceConfig, WaylandClient, LayerSurface};

use crate::event_loop;
use crate::surfaces::ManagedSurface;

/// Messages flowing through the runtime's calloop channel.
pub enum RuntimeMsg {
    /// A subscription produced a message.
    Subscription(SubMessage),
    /// Request a redraw (e.g., from dirty tracking).
    Redraw,
}

/// Run the shell defined in `shell_dir`.
pub fn run(shell_dir: std::path::PathBuf) -> anyhow::Result<()> {
    log::info!("Starting pulpkit v3 with shell dir: {}", shell_dir.display());

    if !shell_dir.is_dir() {
        anyhow::bail!("Shell directory does not exist: {}", shell_dir.display());
    }

    // 1. Lua VM
    let vm = LuaVm::new().map_err(|e| anyhow::anyhow!("Failed to create Lua VM: {e}"))?;
    let lua = vm.lua();

    // 2. Theme
    let theme = Arc::new(load_theme(lua, &shell_dir)?);
    log::info!("Theme loaded (font: {})", theme.font_family);

    // 3. Register Lua APIs
    register_widgets(lua, theme.clone())
        .map_err(|e| anyhow::anyhow!("Failed to register widgets: {e}"))?;
    register_msg_api(lua)
        .map_err(|e| anyhow::anyhow!("Failed to register msg API: {e}"))?;
    register_subscribe_api(lua)
        .map_err(|e| anyhow::anyhow!("Failed to register subscribe API: {e}"))?;

    // 4. Load shell.lua via ElmBridge
    let shell_path = shell_dir.join("shell.lua");
    if !shell_path.exists() {
        anyhow::bail!("shell.lua not found in {}", shell_dir.display());
    }
    let mut bridge = ElmBridge::load(lua, &shell_path)
        .map_err(|e| anyhow::anyhow!("Failed to load shell.lua: {e}"))?;
    log::info!("shell.lua loaded");

    // 5. Call init()
    bridge.init(lua)
        .map_err(|e| anyhow::anyhow!("Failed to call init(): {e}"))?;
    log::info!("init() called");

    // 6. Connect to Wayland
    let mut client = WaylandClient::connect()?;
    log::info!("Connected to Wayland display");

    // Dispatch once to get outputs
    client.event_loop.dispatch(Duration::from_millis(100), &mut client.state)?;
    log::info!("{} output(s) detected", client.state.outputs.len());

    // 7. Call view() to get initial surface list
    let surface_defs = bridge.view(lua)
        .map_err(|e| anyhow::anyhow!("Failed to call view(): {e}"))?;
    log::info!("{} surface(s) defined", surface_defs.len());

    // 8. Create text renderer
    let text_renderer = TextRenderer::new();

    // 9. Create managed surfaces
    let mut surfaces = create_surfaces(&surface_defs, &mut client, &theme)?;

    // 10. Set up calloop channels
    let (msg_sender, msg_channel) = channel::channel::<RuntimeMsg>();
    client.event_loop.handle()
        .insert_source(msg_channel, |event, _, _state| {
            if let channel::Event::Msg(msg) = event {
                match msg {
                    RuntimeMsg::Redraw => log::debug!("Wake: Redraw requested"),
                    RuntimeMsg::Subscription(_) => {}
                }
            }
        })
        .map_err(|e| anyhow::anyhow!("Failed to insert msg channel: {e}"))?;

    // 11. Set up subscription manager
    let (sub_sender, sub_channel) = channel::channel::<SubMessage>();
    let sub_sender_for_loop = msg_sender.clone();
    client.event_loop.handle()
        .insert_source(sub_channel, move |event, _, _state| {
            if let channel::Event::Msg(sub_msg) = event {
                let _ = sub_sender_for_loop.send(RuntimeMsg::Subscription(sub_msg));
            }
        })
        .map_err(|e| anyhow::anyhow!("Failed to insert sub channel: {e}"))?;

    let mut sub_manager = SubscriptionManager::new(sub_sender);

    // 12. Call subscribe() and start initial subscriptions
    if let Ok(sub_defs) = bridge.subscribe(lua) {
        for sub_def in sub_defs {
            start_subscription(&mut sub_manager, &sub_def, &client.event_loop.handle());
        }
    }

    // 13. Initial render
    for surface in &mut surfaces {
        surface.render(&text_renderer, &theme, None);
    }

    // 14. Enter event loop
    event_loop::run(
        &mut client,
        &mut surfaces,
        &mut bridge,
        &mut sub_manager,
        &msg_sender,
        lua,
        &text_renderer,
        &theme,
    )
}

fn create_surfaces(
    defs: &[SurfaceDef],
    client: &mut WaylandClient,
    theme: &Theme,
) -> anyhow::Result<Vec<ManagedSurface>> {
    let mut surfaces = Vec::new();
    for def in defs {
        if def.kind == SurfaceKind::Popup {
            // Popups created on demand, not at startup
            continue;
        }

        let anchor = match def.anchor.as_str() {
            "top" => Anchor::Top,
            "bottom" => Anchor::Bottom,
            "left" => Anchor::Left,
            "right" => Anchor::Right,
            _ => Anchor::Top,
        };

        let height = def.height.unwrap_or(36);
        let width = def.width.unwrap_or(0); // 0 = stretch to fill

        let exclusive_zone = if def.exclusive { height as i32 } else { -1 };

        let config = SurfaceConfig {
            width,
            height,
            anchor,
            layer: Layer::Top,
            exclusive_zone,
            namespace: format!("pulpkit-{}", def.name),
            output: None, // TODO: handle MonitorTarget
            margins: Default::default(),
        };

        let layer_surface = LayerSurface::new(&mut client.state, config)?;
        surfaces.push(ManagedSurface {
            def: def.clone(),
            surface: layer_surface,
            layout: None,
            dirty: true,
            frame_ready: true, // Allow initial render
        });

        log::info!("Created surface: {} ({})", def.name, def.anchor);
    }
    Ok(surfaces)
}

fn start_subscription(
    manager: &mut SubscriptionManager,
    sub_def: &pulpkit_lua::SubscriptionDef,
    handle: &calloop::LoopHandle<'static, pulpkit_wayland::AppState>,
) {
    use pulpkit_lua::SubscriptionDef;
    match sub_def {
        SubscriptionDef::Interval { ms, msg_name } => {
            manager.start_interval(*ms, msg_name.clone(), handle);
        }
        SubscriptionDef::Timeout { ms, msg_name } => {
            manager.start_timeout(*ms, msg_name.clone(), handle);
        }
        // Stream, exec, ipc, etc. will be implemented in later tasks
        other => {
            log::warn!("Subscription type {:?} not yet implemented", other);
        }
    }
}

fn load_theme(lua: &mlua::Lua, shell_dir: &Path) -> anyhow::Result<Theme> {
    let theme_path = shell_dir.join("theme.lua");
    if !theme_path.exists() {
        log::info!("No theme.lua found, using default slate theme");
        return Ok(Theme::default_slate());
    }

    let code = std::fs::read_to_string(&theme_path)?;
    let table: mlua::Table = lua.load(&code).eval()
        .map_err(|e| anyhow::anyhow!("Failed to load theme.lua: {e}"))?;

    let font_family: String = table.get("font_family").unwrap_or_else(|_| "sans-serif".into());
    let font_size: f32 = table.get("font_size").unwrap_or(14.0);

    let mut colors = std::collections::HashMap::new();
    if let Ok(color_table) = table.get::<mlua::Table>("colors") {
        for pair in color_table.pairs::<String, String>() {
            if let Ok((name, hex)) = pair {
                if let Some(color) = pulpkit_render::Color::from_hex(&hex) {
                    colors.insert(name, color);
                }
            }
        }
    }

    Ok(Theme { font_family, font_size, colors })
}
