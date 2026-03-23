//! Event loop — dispatches events, calls Elm lifecycle, renders.

use std::sync::Arc;
use std::time::Duration;

use calloop::channel::Sender;
use mlua::Lua;
use pulpkit_layout::element::Message;
use pulpkit_layout::Theme;
use pulpkit_lua::ElmBridge;
use pulpkit_render::TextRenderer;
use pulpkit_sub::{SubMessage, SubscriptionManager};
use pulpkit_wayland::{InputEvent, WaylandClient};

use crate::runtime::RuntimeMsg;
use crate::surfaces::ManagedSurface;

/// Run the main event loop. Returns when the compositor requests exit.
pub fn run(
    client: &mut WaylandClient,
    surfaces: &mut Vec<ManagedSurface>,
    bridge: &mut ElmBridge,
    sub_manager: &mut SubscriptionManager,
    msg_sender: &Sender<RuntimeMsg>,
    lua: &Lua,
    text_renderer: &TextRenderer,
    theme: &Theme,
) -> anyhow::Result<()> {
    log::info!("Entering event loop");

    let mut hovered_node: Option<usize> = None;
    let mut msg_batch: Vec<Message> = Vec::new();

    loop {
        // 1. Dispatch calloop — blocks until events arrive or timeout
        let timeout = Duration::from_secs(60); // Idle timeout
        client.event_loop.dispatch(timeout, &mut client.state)?;

        // 2. Check for frame callbacks
        if !client.state.frame_callbacks.is_empty() {
            let callbacks: Vec<_> = client.state.frame_callbacks.drain(..).collect();
            for surface_id in &callbacks {
                for surface in surfaces.iter_mut() {
                    if surface.surface.surface_id() == *surface_id {
                        surface.frame_ready = true;
                    }
                }
            }
        }

        // 3. Handle configure events
        if !client.state.pending_configures.is_empty() {
            let configures: Vec<_> = client.state.pending_configures.drain(..).collect();
            for configure in configures {
                for surface in surfaces.iter_mut() {
                    if surface.surface.surface_id() == configure.surface_id {
                        if configure.width > 0 && configure.height > 0 {
                            surface.surface.resize(configure.width, configure.height);
                        }
                        surface.mark_dirty();
                        break;
                    }
                }
            }
        }

        // 4. Process input events → messages
        if !client.state.input_events.is_empty() {
            let input_events: Vec<_> = client.state.input_events.drain(..).collect();
            for event in &input_events {
                match event {
                    InputEvent::PointerMotion { x, y, surface_id, .. } => {
                        // Update hover state
                        for surface in surfaces.iter_mut() {
                            if surface.surface.surface_id() == *surface_id {
                                if let Some(ref layout) = surface.layout {
                                    let (new_hover, damage) = crate::hover::update_hover(
                                        layout, *x, *y, hovered_node,
                                    );
                                    if new_hover != hovered_node {
                                        hovered_node = new_hover;
                                        surface.mark_dirty();
                                    }
                                }
                                break;
                            }
                        }
                    }
                    InputEvent::PointerButton { x, y, surface_id, button: 0x110, pressed: true } => {
                        // Left click — hit test and dispatch
                        for surface in surfaces.iter() {
                            if surface.surface.surface_id() == *surface_id {
                                if let Some(ref layout) = surface.layout {
                                    if let Some(node_idx) = pulpkit_layout::hit_test(layout, *x as f32, *y as f32) {
                                        // Find the element and its on_click message
                                        // For now, walk the flat element list
                                        if let Some(msg) = find_click_msg(&surface.def.root, node_idx) {
                                            msg_batch.push(msg);
                                        }
                                    }
                                }
                                break;
                            }
                        }
                    }
                    InputEvent::PointerLeave { .. } => {
                        hovered_node = None;
                        for surface in surfaces.iter_mut() {
                            surface.mark_dirty();
                        }
                    }
                    _ => {}
                }
            }
        }

        // 5. Process subscription messages
        // (These arrive via the calloop channel and are already dispatched)
        // We collect any pending sub messages via polling the sender side
        // Actually, sub messages arrive via the calloop channel callback which
        // sends RuntimeMsg::Subscription. Since we're using calloop dispatch,
        // we need a different approach — store pending sub msgs.
        // For now, timer callbacks directly produce messages via the sender.

        // 6. Process message batch through Elm lifecycle
        if !msg_batch.is_empty() {
            for msg in msg_batch.drain(..) {
                if let Err(e) = bridge.update(lua, &msg) {
                    log::error!("Lua update() error: {e}");
                }
            }

            // Call view() and update surfaces
            match bridge.view(lua) {
                Ok(new_defs) => {
                    // Update root elements for existing surfaces
                    for surface in surfaces.iter_mut() {
                        for def in &new_defs {
                            if def.name == surface.def.name {
                                if surface.def.root != def.root {
                                    surface.def = def.clone();
                                    surface.mark_dirty();
                                }
                                break;
                            }
                        }
                    }
                }
                Err(e) => {
                    log::error!("Lua view() error: {e}");
                    // Keep previous tree
                }
            }
        }

        // 7. Render dirty surfaces (gated on frame_ready)
        for surface in surfaces.iter_mut() {
            if surface.dirty && surface.frame_ready {
                surface.render(text_renderer, theme, hovered_node);
                // Request next frame callback
                surface.surface.request_frame(&client.state.qh);
            }
        }

        // 8. Exit check
        if client.state.exit_requested {
            log::info!("Exit requested by compositor");
            break;
        }
    }

    Ok(())
}

/// Walk the element tree to find an on_click message at the given hit index.
/// This is a simplified version — proper index mapping will be refined.
fn find_click_msg(root: &pulpkit_layout::Element, _target_idx: usize) -> Option<Message> {
    // For now, walk all buttons and return the first one found
    // TODO: proper element-to-layout-node mapping
    find_click_in_element(root)
}

fn find_click_in_element(element: &pulpkit_layout::Element) -> Option<Message> {
    match element {
        pulpkit_layout::Element::Button { on_click, children, .. } => {
            if let Some(msg) = on_click {
                return Some(msg.clone());
            }
            for child in children {
                if let Some(msg) = find_click_in_element(child) {
                    return Some(msg);
                }
            }
            None
        }
        pulpkit_layout::Element::Container { children, .. } => {
            for child in children {
                if let Some(msg) = find_click_in_element(child) {
                    return Some(msg);
                }
            }
            None
        }
        _ => None,
    }
}
