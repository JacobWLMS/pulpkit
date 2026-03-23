//! Event loop — single iteration dispatches events, fires timers, renders.

use std::time::Duration;

use mlua::prelude::*;
use pulpkit_layout::Theme;
use pulpkit_layout::tree::{InteractiveKind, Node};
use pulpkit_reactive::ReactiveRuntime;
use pulpkit_render::TextRenderer;
use pulpkit_wayland::{AppState, InputEvent, WaylandClient};

use crate::events::{self, ClickResult};
use crate::popups::{self, ManagedPopup};
use crate::surfaces::ManagedSurface;
use crate::timers::{self, ActiveTimer};

/// Run the main event loop. Returns when the compositor requests exit.
pub fn run(
    client: &mut WaylandClient,
    surfaces: &mut Vec<ManagedSurface>,
    popups: &mut Vec<ManagedPopup>,
    timers: &mut Vec<ActiveTimer>,
    cancelled_timers: &pulpkit_lua::CancelledTimers,
    ipc_commands: &std::rc::Rc<std::cell::RefCell<Vec<String>>>,
    stream_events: &std::rc::Rc<std::cell::RefCell<Vec<(u64, String)>>>,
    stream_callbacks: &std::collections::HashMap<u64, mlua::RegistryKey>,
    lua: &Lua,
    text_renderer: &TextRenderer,
    theme: &Theme,
    rt: &ReactiveRuntime,
) -> anyhow::Result<()> {
    log::info!("Entering event loop");

    // Slider drag state: when a slider is being dragged, we track it here
    // and update on every PointerMotion until mouse-up.
    struct SliderDrag {
        value: pulpkit_reactive::Signal<pulpkit_reactive::DynValue>,
        on_change: Option<std::rc::Rc<dyn Fn(f64)>>,
        min: f64,
        max: f64,
        node_x: f32,
        node_width: f32,
        // Is this in a popup? If so, store the popup offset for coord translation.
        popup_offset_x: f32,
    }
    let mut active_drag: Option<SliderDrag> = None;

    loop {
        // Compute dispatch timeout: animate at 60fps, otherwise sleep until
        // the next interval or 60 seconds.
        let animating = false; // No popup animations in single-surface model.
        let timeout = if animating {
            Duration::from_millis(16) // ~60fps
        } else {
            timers::next_timer_timeout(timers, Duration::from_secs(60))
        };

        client
            .event_loop
            .dispatch(timeout, &mut client.state)?;

        // --- Handle configure events (surface resize) ---
        if !client.state.pending_configures.is_empty() {
            let configures: Vec<_> = client.state.pending_configures.drain(..).collect();
            for configure in configures {
                for managed in surfaces.iter_mut() {
                    if managed.surface.surface_id() == configure.surface_id {
                        if configure.width > 0 && configure.height > 0 {
                            managed.surface.resize(configure.width, configure.height);
                        }
                        managed.mark_dirty();
                        break;
                    }
                }
            }
        }

        // --- Drain file watcher (hot-reload disabled — Wave 2) ---
        // crate::watcher events are drained in runtime.rs before entering the loop.

        // --- Dispatch input events ---
        let mut any_handler_fired = false;
        if !client.state.input_events.is_empty() {
            let input_events: Vec<_> = client.state.input_events.drain(..).collect();
            for event in &input_events {
                match event {
                    InputEvent::PointerMotion {
                        x, y, surface_id, ..
                    } => {
                        // Handle slider drag.
                        if let Some(ref drag) = active_drag {
                            let fx = *x as f32 - drag.popup_offset_x;
                            let ratio = ((fx - drag.node_x) / drag.node_width).clamp(0.0, 1.0) as f64;
                            let new_val = drag.min + (drag.max - drag.min) * ratio;
                            drag.value.set(pulpkit_reactive::DynValue::Float(new_val));
                            if let Some(ref cb) = drag.on_change {
                                cb(new_val);
                            }
                            any_handler_fired = true;
                        } else {
                            events::dispatch_hover(surfaces, *x, *y, surface_id);
                            update_cursor(surfaces, &mut client.state);
                        }
                    }
                    InputEvent::PointerLeave { surface_id, .. } => {
                        events::dispatch_leave(surfaces, surface_id);
                        client.state.set_cursor("default");
                    }
                    InputEvent::PointerButton {
                        button: 0x110,
                        pressed: false,
                        ..
                    } => {
                        // Mouse up — end any active slider drag.
                        if active_drag.is_some() {
                            active_drag = None;
                            any_handler_fired = true;
                        }
                    }
                    InputEvent::PointerButton {
                        surface_id,
                        x,
                        y,
                        button,
                        pressed: true,
                    } => {
                        if *button == 0x110 {
                            let fx = *x as f32;
                            let fy = *y as f32;

                            // Check if click is on a popup surface.
                            let mut popup_clicked = false;
                            for popup in popups.iter_mut() {
                                if popup.surface_id().as_ref() == Some(surface_id) {
                                    if let Some(ref layout) = popup.layout {
                                        if let Some((val, min, max, on_change, nx, nw)) =
                                            events::find_slider_at(layout, fx, fy)
                                        {
                                            active_drag = Some(SliderDrag {
                                                value: val, on_change, min, max,
                                                node_x: nx, node_width: nw,
                                                popup_offset_x: 0.0, // popup coords are already local
                                            });
                                        }
                                        let result = events::dispatch_click_on_layout(layout, fx, fy);
                                        if result == ClickResult::Handled {
                                            any_handler_fired = true;
                                        }
                                    }
                                    popup_clicked = true;
                                    break;
                                }
                            }

                            // Click on bar surface — dismiss popups, dispatch to bar.
                            if !popup_clicked {
                                for popup in popups.iter_mut() {
                                    if popup.should_be_visible() && popup.config.dismiss_on_outside {
                                        popup.dismiss();
                                        any_handler_fired = true;
                                    }
                                }
                                let result = events::dispatch_click(
                                    surfaces, popups, *x, *y, surface_id,
                                );
                                if result == ClickResult::Handled {
                                    any_handler_fired = true;
                                }
                            }
                        }
                    }
                    InputEvent::PointerAxis {
                        x,
                        y,
                        delta,
                        horizontal: false,
                        surface_id,
                        ..
                    } => {
                        let scroll_up = *delta < 0.0;
                        if events::dispatch_scroll(
                            surfaces, popups, *x, *y, surface_id, scroll_up, *delta,
                        ) {
                            any_handler_fired = true;
                        }
                    }
                    InputEvent::KeyPress {
                        keysym,
                        utf8,
                        ..
                    } => {
                        // Dispatch key to the first visible popup with an on_key handler.
                        for popup in popups.iter() {
                            if popup.should_be_visible() {
                                if let Some(ref key) = popup.on_key {
                                    let cb: mlua::Function = lua.registry_value(key).unwrap();
                                    let key_name = keysym_to_name(*keysym);
                                    let _ = cb.call::<()>((
                                        key_name,
                                        utf8.clone().unwrap_or_default(),
                                    ));
                                    any_handler_fired = true;
                                }
                                break;
                            }
                        }
                    }
                    InputEvent::KeyboardLeave { .. } => {
                        // Keyboard focus lost — dismiss all dismissable popups.
                        for popup in popups.iter_mut() {
                            if popup.should_be_visible() && popup.config.dismiss_on_outside {
                                popup.dismiss();
                                any_handler_fired = true;
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        // --- Process IPC commands (arrive via calloop channel, no polling needed) ---
        {
            let cmds: Vec<String> = ipc_commands.borrow_mut().drain(..).collect();
            for cmd in cmds {
                log::info!("IPC command: {}", cmd);
                if let Err(e) = lua.load(&cmd).exec() {
                    log::error!("IPC command error: {e}");
                }
                any_handler_fired = true;
            }
        }

        // --- Dispatch exec_stream() output lines to Lua callbacks ---
        // NOTE: don't set any_handler_fired here — stream callbacks update
        // signals, which trigger dirty-tracking Effects automatically.
        // Force-rendering on every stream line would be wasteful.
        {
            let events: Vec<(u64, String)> = stream_events.borrow_mut().drain(..).collect();
            for (stream_id, line) in events {
                if let Some(cb_key) = stream_callbacks.get(&stream_id) {
                    if let Ok(cb) = lua.registry_value::<mlua::Function>(cb_key) {
                        if let Err(e) = cb.call::<()>(line) {
                            log::error!("Stream {} callback error: {e}", stream_id);
                        }
                    }
                }
            }
        }

        // --- Process timer cancellations from Lua ---
        {
            let mut cancelled = cancelled_timers.borrow_mut();
            for id in cancelled.drain(..) {
                if let Some(timer) = timers.iter_mut().find(|t| t.id == id) {
                    timer.cancelled = true;
                }
            }
        }

        // --- Fire timer callbacks ---
        if timers::fire_due_timers(timers, lua) {
            any_handler_fired = true;
        }

        // --- Flush reactive effects ---
        // Signal changes from handlers/intervals queue effects. Flush them now
        // so that Effects (including dirty-marking effects) execute before render.
        rt.flush();

        // --- Show/hide popups via xdg_popup creation/destruction ---
        {
            let bar_surface = surfaces.first();
            let bar_w = bar_surface.map(|s| s.surface.width()).unwrap_or(1920);
            let bar_h = bar_surface.map(|s| s.surface.height()).unwrap_or(40);
            for popup in popups.iter_mut() {
                let wants = popup.should_be_visible();
                let has = popup.surface.is_some();
                if wants && !has {
                    if let Some(bar) = surfaces.first() {
                        popup.show(&mut client.state, &bar.surface, bar_w, bar_h, text_renderer, theme);
                    }
                } else if !wants && has {
                    popup.hide();
                }
            }
        }

        // Handle popup_done events (compositor dismissed popup via xdg protocol)
        if !client.state.popup_done_ids.is_empty() {
            let done_ids: Vec<_> = client.state.popup_done_ids.drain(..).collect();
            for id in &done_ids {
                for popup in popups.iter_mut() {
                    if popup.surface_id().as_ref() == Some(id) {
                        popup.dismiss();
                        any_handler_fired = true;
                    }
                }
            }
        }

        // If any handler or interval fired, mark bar dirty.
        if any_handler_fired {
            for surface in surfaces.iter() {
                surface.mark_dirty();
            }
        }

        // --- Render bar ---
        for surface in surfaces.iter_mut() {
            if surface.dirty.get() {
                surface.render(text_renderer, theme);
            }
        }

        // --- Handle popup configure events ---
        if !client.state.popup_configured_ids.is_empty() {
            let configured_ids: Vec<_> = client.state.popup_configured_ids.drain(..).collect();
            for id in &configured_ids {
                for popup in popups.iter_mut() {
                    if let Some(ref mut surface) = popup.surface {
                        if surface.surface_id() == *id {
                            surface.configured = true;
                        }
                    }
                }
            }
        }

        // --- Render visible popups (only after configured) ---
        for popup in popups.iter_mut() {
            if let Some(ref surface) = popup.surface {
                if surface.configured {
                    popup.render(text_renderer, theme);
                }
            }
        }

        // --- Exit check ---
        if client.state.exit_requested {
            log::info!("Exit requested by compositor");
            break;
        }
    }

    Ok(())
}

/// Set the pointer cursor based on the currently hovered widget type.
fn update_cursor(surfaces: &[ManagedSurface], app_state: &mut AppState) {
    for surface in surfaces {
        if let Some(idx) = surface.hovered_node {
            if let Some(ref layout) = surface.layout {
                if let Some(node) = layout.nodes.get(idx) {
                    let cursor = match &node.source_node {
                        Node::Interactive { kind: InteractiveKind::Button { .. }, .. } => "pointer",
                        Node::Interactive { kind: InteractiveKind::Slider { .. }, .. } => "col-resize",
                        Node::Interactive { kind: InteractiveKind::Toggle { .. }, .. } => "pointer",
                        _ => "default",
                    };
                    app_state.set_cursor(cursor);
                    return;
                }
            }
        }
    }
    app_state.set_cursor("default");
}

/// Map common XKB keysyms to human-readable names for Lua.
fn keysym_to_name(keysym: u32) -> String {
    match keysym {
        0xff08 => "BackSpace".into(),
        0xff09 => "Tab".into(),
        0xff0d => "Return".into(),
        0xff1b => "Escape".into(),
        0xffff => "Delete".into(),
        0xff50 => "Home".into(),
        0xff51 => "Left".into(),
        0xff52 => "Up".into(),
        0xff53 => "Right".into(),
        0xff54 => "Down".into(),
        0xff55 => "Page_Up".into(),
        0xff56 => "Page_Down".into(),
        0xff57 => "End".into(),
        0x20 => "space".into(),
        k if (0x20..=0x7e).contains(&k) => {
            String::from(char::from_u32(k).unwrap_or('?'))
        }
        other => format!("0x{other:04x}"),
    }
}
