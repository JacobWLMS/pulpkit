//! Runtime orchestration — setup, Lua loading, surface creation, event loop entry.

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use calloop::channel;
use pulpkit_lua::{
    LuaVm, register_timer_api, register_popup_fn, register_signal_api, register_widgets,
    register_window_fn,
};
use pulpkit_reactive::ReactiveRuntime;
use pulpkit_render::TextRenderer;
use pulpkit_wayland::WaylandClient;

use crate::event_loop;
use crate::setup;
use crate::theme::load_theme;
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
    // 0. File watcher disabled — causes spurious events from Lua I/O (Wave 2 fix).
    // let watcher = watcher::FileWatcher::new(shell_dir)?;
    // while watcher.poll().is_some() {}

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
    let stream_registry = pulpkit_lua::StreamRegistry::default();
    pulpkit_lua::register_system_api(lua, stream_registry.clone())
        .map_err(|e| anyhow::anyhow!("Failed to register system API: {e}"))?;

    let window_registry = pulpkit_lua::WindowRegistry::default();
    register_window_fn(lua, window_registry.clone())
        .map_err(|e| anyhow::anyhow!("Failed to register window fn: {e}"))?;

    let popup_registry = pulpkit_lua::PopupRegistry::default();
    register_popup_fn(lua, popup_registry.clone())
        .map_err(|e| anyhow::anyhow!("Failed to register popup fn: {e}"))?;

    let timer_registry = pulpkit_lua::TimerRegistry::default();
    let cancelled_timers = pulpkit_lua::CancelledTimers::default();
    register_timer_api(lua, timer_registry.clone(), cancelled_timers.clone())
        .map_err(|e| anyhow::anyhow!("Failed to register timer API: {e}"))?;

    // 4. Execute shell.lua.
    let shell_path = shell_dir.join("shell.lua");
    if !shell_path.exists() {
        anyhow::bail!("shell.lua not found in {}", shell_dir.display());
    }
    // Set shell_dir global so Lua can find lib.lua etc.
    lua.globals().set("shell_dir", shell_dir.to_string_lossy().as_ref())
        .map_err(|e| anyhow::anyhow!("Failed to set shell_dir: {e}"))?;
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

    // IPC socket — commands arrive via calloop channel, waking the loop.
    let ipc_commands: std::rc::Rc<std::cell::RefCell<Vec<String>>> =
        std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let _ipc_socket_path;
    if let Some((ipc_channel, path)) = crate::ipc::start_ipc_server() {
        _ipc_socket_path = Some(path);
        let cmds = ipc_commands.clone();
        client
            .event_loop
            .handle()
            .insert_source(ipc_channel, move |event, _, _state| {
                if let channel::Event::Msg(cmd) = event {
                    cmds.borrow_mut().push(cmd);
                }
            })
            .map_err(|e| anyhow::anyhow!("Failed to insert IPC channel: {e}"))?;
    } else {
        _ipc_socket_path = None;
    }

    client
        .event_loop
        .dispatch(Duration::from_millis(100), &mut client.state)?;
    log::info!("{} output(s) detected", client.state.outputs.len());

    // 6. Create text renderer.
    let text_renderer = TextRenderer::new();

    // 7. Create surfaces, popups, intervals.
    let mut surfaces = setup::create_surfaces(&window_defs, lua, &mut client, &text_renderer, &theme)?;
    drop(window_defs);

    crate::dirty::wire_dirty_tracking(&surfaces, &wake_sender);

    let mut popups = setup::create_popups(&popup_registry.borrow(), lua, &client)?;
    let mut timers = setup::create_timers(&timer_registry.borrow(), lua)?;

    // 7b. Spawn exec_stream() subprocesses.
    // Stream events arrive via calloop channel, waking the loop immediately.
    let stream_events: std::rc::Rc<std::cell::RefCell<Vec<(u64, String)>>> =
        std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let mut stream_callbacks: std::collections::HashMap<u64, mlua::RegistryKey> =
        std::collections::HashMap::new();
    {
        let stream_defs = stream_registry.borrow();
        if !stream_defs.is_empty() {
            let (stream_sender, stream_channel) = channel::channel::<(u64, String)>();
            let evts = stream_events.clone();
            client
                .event_loop
                .handle()
                .insert_source(stream_channel, move |event, _, _state| {
                    if let channel::Event::Msg(msg) = event {
                        evts.borrow_mut().push(msg);
                    }
                })
                .map_err(|e| anyhow::anyhow!("Failed to insert stream channel: {e}"))?;

            for def in stream_defs.iter() {
                let id = def.id;
                let cmd = def.cmd.clone();
                let sender = stream_sender.clone();

                // Clone the callback key for the event loop.
                let cb_key = lua
                    .registry_value::<mlua::Function>(&def.callback_key)
                    .and_then(|f| lua.create_registry_value(f))
                    .map_err(|e| anyhow::anyhow!("Failed to clone stream callback: {e}"))?;
                stream_callbacks.insert(id, cb_key);

                // Spawn background thread: runs command, reads stdout line-by-line.
                std::thread::spawn(move || {
                    use std::io::BufRead;
                    let child = std::process::Command::new("sh")
                        .arg("-c")
                        .arg(&cmd)
                        .stdout(std::process::Stdio::piped())
                        .stderr(std::process::Stdio::null())
                        .spawn();

                    match child {
                        Ok(mut child) => {
                            if let Some(stdout) = child.stdout.take() {
                                let reader = std::io::BufReader::new(stdout);
                                for line in reader.lines() {
                                    match line {
                                        Ok(line) => {
                                            if sender.send((id, line)).is_err() {
                                                break; // channel closed
                                            }
                                        }
                                        Err(_) => break,
                                    }
                                }
                            }
                            let _ = child.wait();
                        }
                        Err(e) => {
                            log::error!("exec_stream failed to spawn '{}': {e}", cmd);
                        }
                    }
                    log::info!("Stream {} ('{}') ended", id, cmd);
                });

                log::info!("Stream {} spawned: {}", id, def.cmd);
            }
        }
    }

    // 8. Enter the event loop.
    event_loop::run(
        &mut client,
        &mut surfaces,
        &mut popups,
        &mut timers,
        &cancelled_timers,
        &ipc_commands,
        &stream_events,
        &stream_callbacks,
        lua,
        &text_renderer,
        &theme,
        rt,
    )
}
