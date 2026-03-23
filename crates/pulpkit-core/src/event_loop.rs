//! Event loop — single iteration dispatches events, fires timers, renders.

use std::time::Duration;

use mlua::prelude::*;
use pulpkit_layout::Theme;
use pulpkit_layout::tree::{InteractiveKind, Node};
use pulpkit_reactive::ReactiveRuntime;
use pulpkit_render::TextRenderer;
use pulpkit_wayland::{AppState, InputEvent, WaylandClient};

use crate::events::{self, ClickResult};
use crate::popups::{ManagedPopup, PopupState};
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
        let animating = popups.iter().any(|p| p.state.is_animating());
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
                // Bar surfaces
                for managed in surfaces.iter_mut() {
                    if managed.surface.surface_id() == configure.surface_id {
                        if configure.width > 0 && configure.height > 0 {
                            managed.surface.resize(configure.width, configure.height);
                        }
                        managed.mark_dirty(); // always render after configure (acks the configure)
                        break;
                    }
                }
                // Popup surfaces
                for popup in popups.iter_mut() {
                    if popup.surface_id() == Some(configure.surface_id.clone()) {
                        popup.handle_configure(
                            configure.width,
                            configure.height,
                            text_renderer,
                            theme,
                        );
                        break;
                    }
                }
                // Backdrop surfaces — commit transparent buffer on configure.
                for popup in popups.iter_mut() {
                    if let Some(ref mut bd) = popup.backdrop {
                        if bd.surface_id() == configure.surface_id {
                            if configure.width > 0 && configure.height > 0 {
                                bd.resize(configure.width, configure.height);
                            }
                            // Fill with transparent pixels and commit.
                            let buf = bd.get_buffer();
                            for b in buf.iter_mut() { *b = 0; }
                            bd.commit();
                            break;
                        }
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
                        // Dismiss popups when pointer leaves their surface.
                        for popup in popups.iter_mut() {
                            if popup.surface_id().as_ref() == Some(surface_id) {
                                if popup.config.dismiss_on_outside {
                                    popup.dismiss();
                                    any_handler_fired = true;
                                }
                            }
                        }
                    }
                    InputEvent::PointerButton {
                        surface_id,
                        x,
                        y,
                        button,
                        pressed: true,
                    } => {
                        // BTN_LEFT = 0x110
                        if *button == 0x110 {
                            // Check if click was on a backdrop → dismiss that popup.
                            let mut backdrop_click = false;
                            for popup in popups.iter_mut() {
                                if let Some(ref bd) = popup.backdrop {
                                    if bd.surface_id() == *surface_id {
                                        popup.dismiss();
                                        backdrop_click = true;
                                        any_handler_fired = true;
                                        break;
                                    }
                                }
                            }

                            if !backdrop_click {
                                dismiss_popups_on_outside_click(popups, surface_id);
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
                        surface_id,
                        keysym,
                        utf8,
                        ..
                    } => {
                        // Dispatch key to the popup that owns this surface.
                        for popup in popups.iter() {
                            if popup.surface_id().as_ref() == Some(surface_id) {
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
                    InputEvent::KeyboardLeave { surface_id } => {
                        // Keyboard focus lost on a popup → dismiss if dismiss_on_outside.
                        for popup in popups.iter_mut() {
                            if popup.surface_id().as_ref() == Some(surface_id) {
                                if popup.config.dismiss_on_outside {
                                    popup.dismiss();
                                    any_handler_fired = true;
                                }
                                break;
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

        // --- Check popup visibility signals ---
        let bar_info = surfaces.first().map(|s| {
            (s.surface.height(), s.surface.width())
        });
        let click_x = client.state.pointer_position.map(|(x, _)| x).unwrap_or(0.0);
        for popup in popups.iter_mut() {
            let wants_visible = popup.should_be_visible();
            match &popup.state {
                PopupState::Hidden if wants_visible => {
                    let (parent_h, parent_w) = bar_info.unwrap_or((48, 1920));
                    popup.show_at(&mut client.state, parent_h, parent_w, click_x, text_renderer, theme);
                }
                PopupState::Visible { .. } | PopupState::FadingIn { .. }
                    if !wants_visible =>
                {
                    popup.hide();
                }
                _ => {}
            }
        }

        // --- Tick popup animations ---
        for popup in popups.iter_mut() {
            popup.tick(text_renderer, theme);
        }

        // If any handler or interval fired, mark all surfaces dirty
        // (state may have changed anywhere in the reactive graph).
        if any_handler_fired {
            for surface in surfaces.iter() {
                surface.mark_dirty();
            }
            for popup in popups.iter_mut() {
                popup.render_content(text_renderer, theme);
            }
        }

        // --- Single render pass: only dirty surfaces ---
        for surface in surfaces.iter_mut() {
            if surface.dirty.get() {
                surface.render(text_renderer, theme);
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

/// Dismiss popups with dismiss_on_outside when a click misses their surface.
fn dismiss_popups_on_outside_click(
    popups: &mut [ManagedPopup],
    clicked_surface_id: &wayland_client::backend::ObjectId,
) {
    for popup in popups.iter_mut() {
        if !popup.config.dismiss_on_outside {
            continue;
        }
        match &popup.state {
            PopupState::Visible { .. } | PopupState::FadingIn { .. } => {
                let is_on_popup = popup
                    .surface_id()
                    .as_ref()
                    .map(|id| id == clicked_surface_id)
                    .unwrap_or(false);
                if !is_on_popup {
                    popup.dismiss();
                }
            }
            _ => {}
        }
    }
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
