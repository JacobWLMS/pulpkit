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
                        let (old_w, old_h) = (managed.surface.width(), managed.surface.height());
                        if configure.width > 0
                            && configure.height > 0
                            && (configure.width != old_w || configure.height != old_h)
                        {
                            managed.surface.resize(configure.width, configure.height);
                            managed.mark_dirty();
                        }
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
                        // BTN_LEFT = 0x110
                        if *button == 0x110 {
                            // Always dismiss popups on clicks outside the popup surface,
                            // even if the click hit a bar button (the button's handler
                            // runs too — e.g., toggle_popup closes all first).
                            dismiss_popups_on_outside_click(popups, surface_id);

                            let result = events::dispatch_click(
                                surfaces, popups, *x, *y, surface_id,
                            );
                            if result == ClickResult::Handled {
                                any_handler_fired = true;
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
                            surfaces, popups, *x, *y, surface_id, scroll_up,
                        ) {
                            any_handler_fired = true;
                        }
                    }
                    _ => {}
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
