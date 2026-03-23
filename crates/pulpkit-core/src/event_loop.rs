//! Event loop — dispatches events, calls Elm lifecycle, renders.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use mlua::Lua;
use pulpkit_layout::element::Message;
use pulpkit_layout::Theme;
use pulpkit_lua::ElmBridge;
use pulpkit_render::TextRenderer;
use pulpkit_sub::{SubMessage, SubscriptionManager};
use pulpkit_wayland::{InputEvent, WaylandClient};

use crate::surfaces::ManagedSurface;

/// Run the main event loop. Returns when the compositor requests exit.
pub fn run(
    client: &mut WaylandClient,
    surfaces: &mut Vec<ManagedSurface>,
    bridge: &mut ElmBridge,
    _sub_manager: &mut SubscriptionManager,
    pending_sub_msgs: &Rc<RefCell<Vec<SubMessage>>>,
    lua: &Lua,
    text_renderer: &TextRenderer,
    theme: &Theme,
) -> anyhow::Result<()> {
    log::info!("Entering event loop");

    let mut hovered_node: Option<usize> = None;
    let mut msg_batch: Vec<Message> = Vec::new();

    loop {
        // 1. Dispatch calloop — blocks until events arrive or timeout
        let timeout = Duration::from_secs(60);
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
                        for surface in surfaces.iter_mut() {
                            if surface.surface.surface_id() == *surface_id {
                                if let Some(ref layout) = surface.layout {
                                    let (new_hover, _damage) = crate::hover::update_hover(
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
                        for surface in surfaces.iter() {
                            if surface.surface.surface_id() == *surface_id {
                                if let Some(ref layout) = surface.layout {
                                    if let Some(element) = pulpkit_layout::flex::hit_test_element(layout, *x as f32, *y as f32) {
                                        match element {
                                            pulpkit_layout::Element::Button { on_click: Some(msg), .. } => {
                                                msg_batch.push(msg.clone());
                                            }
                                            pulpkit_layout::Element::Toggle { on_toggle: Some(msg), checked, .. } => {
                                                // Toggle sends the inverted value
                                                let mut m = msg.clone();
                                                m.data = Some(pulpkit_layout::MessageData::Bool(!checked));
                                                msg_batch.push(m);
                                            }
                                            _ => {}
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

        // 5. Drain subscription messages → Elm messages
        {
            let sub_msgs: Vec<SubMessage> = pending_sub_msgs.borrow_mut().drain(..).collect();
            for sub_msg in sub_msgs {
                msg_batch.push(Message {
                    msg_type: sub_msg.msg_type,
                    data: sub_msg.data.map(|s| pulpkit_layout::element::MessageData::String(s)),
                });
            }
        }

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
                }
            }
        }

        // 7. Render dirty surfaces (gated on frame_ready)
        for surface in surfaces.iter_mut() {
            if surface.dirty && surface.frame_ready {
                surface.render(text_renderer, theme, hovered_node);
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

