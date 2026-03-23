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
                            // Update screen dimensions from configure (correct for fractional scaling).
                            if managed.expanded {
                                managed.screen_width = configure.width;
                                managed.screen_height = configure.height;
                                // Recompute popup positions with correct dimensions.
                                for popup in popups.iter_mut() {
                                    popup.compute_position(
                                        managed.screen_width,
                                        managed.screen_height,
                                        managed.bar_height,
                                    );
                                }
                            }
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
                        events::dispatch_hover(surfaces, *x, *y, surface_id);
                        update_cursor(surfaces, &mut client.state);
                    }
                    InputEvent::PointerLeave { surface_id, .. } => {
                        events::dispatch_leave(surfaces, surface_id);
                        client.state.set_cursor("default");
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
                            let bar_h = surfaces.first().map(|s| s.bar_height as f32).unwrap_or(40.0);

                            // Check if click is inside any visible popup.
                            let mut popup_clicked = false;
                            for popup in popups.iter_mut() {
                                if popup.should_be_visible() && popup.contains(fx, fy) {
                                    // Click inside popup — dispatch to popup's layout.
                                    let (lx, ly) = popup.to_local(fx, fy);
                                    if let Some(ref layout) = popup.layout {
                                        let result = events::dispatch_click_on_layout(layout, lx, ly);
                                        if result == ClickResult::Handled {
                                            any_handler_fired = true;
                                        }
                                    }
                                    popup_clicked = true;
                                    break;
                                }
                            }

                            if !popup_clicked {
                                // Click outside all popups.
                                if fy <= bar_h {
                                    // Click on bar — dispatch to bar widgets AND dismiss popups.
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
                                } else {
                                    // Click on transparent area — dismiss all popups.
                                    for popup in popups.iter_mut() {
                                        if popup.should_be_visible() && popup.config.dismiss_on_outside {
                                            popup.dismiss();
                                            any_handler_fired = true;
                                        }
                                    }
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

        // --- Expand/shrink surface based on popup visibility ---
        let any_popup_visible = popups::any_visible(popups);
        if let Some(surface) = surfaces.first_mut() {
            if any_popup_visible && !surface.expanded {
                for popup in popups.iter_mut() {
                    popup.compute_position(surface.screen_width, surface.screen_height, surface.bar_height);
                }
                surface.expand();
                // Don't mark dirty yet — wait for configure with new dimensions.
            } else if !any_popup_visible && surface.expanded {
                surface.shrink();
                // Don't mark dirty yet — wait for configure.
            }
        }

        // If any handler or interval fired, mark surfaces dirty.
        if any_handler_fired {
            for surface in surfaces.iter() {
                surface.mark_dirty();
            }
        }

        // --- Single render pass: bar + popups on the same surface ---
        for surface in surfaces.iter_mut() {
            if surface.dirty.get() {
                surface.render_with_popups(popups, text_renderer, theme);
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
