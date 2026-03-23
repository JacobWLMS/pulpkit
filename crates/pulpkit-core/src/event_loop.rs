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

    // Slider drag state
    struct SliderDrag {
        on_change: Message,
        min: f64,
        max: f64,
        node_x: f32,
        node_width: f32,
    }
    let mut active_drag: Option<SliderDrag> = None;

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
                        surface.configured = true;
                        surface.mark_dirty();
                        log::debug!("Surface configured: {} ({}x{})",
                            surface.name(), configure.width, configure.height);
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
                        // Handle slider drag — only keep latest value
                        if let Some(ref drag) = active_drag {
                            let fx = *x as f32;
                            let ratio = ((fx - drag.node_x) / drag.node_width).clamp(0.0, 1.0) as f64;
                            let new_val = drag.min + (drag.max - drag.min) * ratio;
                            let mut msg = drag.on_change.clone();
                            msg.data = Some(pulpkit_layout::MessageData::Float(new_val));
                            // Replace any existing drag msg to avoid flooding
                            msg_batch.retain(|m| m.msg_type != msg.msg_type);
                            msg_batch.push(msg);
                        } else {
                            // Normal hover tracking
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
                    }
                    InputEvent::PointerButton { x, y, surface_id, button: 0x110, pressed: true } => {
                        // Check if click is on a window surface — dismiss popups
                        let clicked_on_window = surfaces.iter().any(|s|
                            s.surface.surface_id() == *surface_id
                            && s.def.kind == pulpkit_layout::SurfaceKind::Window
                        );
                        if clicked_on_window {
                            let has_dismissable = surfaces.iter().any(|s|
                                s.def.kind == pulpkit_layout::SurfaceKind::Popup
                                && s.def.dismiss_on_outside
                            );
                            if has_dismissable {
                                msg_batch.push(Message {
                                    msg_type: "dismiss".into(),
                                    data: None,
                                });
                            }
                        }

                        for surface in surfaces.iter() {
                            if surface.surface.surface_id() == *surface_id {
                                if let Some(ref layout) = surface.layout {
                                    if let Some((idx, element)) = pulpkit_layout::flex::hit_test_interactive(layout, *x as f32, *y as f32) {
                                        match element {
                                            pulpkit_layout::Element::Button { on_click: Some(msg), .. } => {
                                                msg_batch.push(msg.clone());
                                            }
                                            pulpkit_layout::Element::Toggle { on_toggle: Some(msg), checked, .. } => {
                                                let mut m = msg.clone();
                                                m.data = Some(pulpkit_layout::MessageData::Bool(!checked));
                                                msg_batch.push(m);
                                            }
                                            pulpkit_layout::Element::Slider { on_change: Some(msg), min, max, .. } => {
                                                let node = &layout.nodes[idx];
                                                    active_drag = Some(SliderDrag {
                                                        on_change: msg.clone(),
                                                        min: *min, max: *max,
                                                        node_x: node.x, node_width: node.width,
                                                    });
                                                    // Also send the initial click position value
                                                    let ratio = ((*x as f32 - node.x) / node.width).clamp(0.0, 1.0) as f64;
                                                    let val = min + (max - min) * ratio;
                                                    let mut m = msg.clone();
                                                    m.data = Some(pulpkit_layout::MessageData::Float(val));
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
                    InputEvent::PointerButton { button: 0x110, pressed: false, .. } => {
                        // Mouse up — end slider drag
                        if active_drag.is_some() {
                            active_drag = None;
                        }
                    }
                    InputEvent::PointerAxis { x: _, y: _, surface_id, .. } => {
                        // Scroll events — mark surface dirty for hover update
                        for surface in surfaces.iter_mut() {
                            if surface.surface.surface_id() == *surface_id {
                                // Scroll handling will be added when scroll containers are used
                                break;
                            }
                        }
                    }
                    InputEvent::PointerLeave { .. } => {
                        hovered_node = None;
                        active_drag = None;
                        for surface in surfaces.iter_mut() {
                            surface.mark_dirty();
                        }
                    }
                    InputEvent::KeyPress { keysym, utf8, .. } => {
                        // Dispatch key events to any popup with keyboard focus
                        // or to a focused Input widget
                        let key_name = keysym_to_name(*keysym);
                        msg_batch.push(Message {
                            msg_type: "key".into(),
                            data: Some(pulpkit_layout::MessageData::Table(vec![
                                ("key".into(), pulpkit_layout::MessageData::String(key_name)),
                                ("text".into(), pulpkit_layout::MessageData::String(
                                    utf8.clone().unwrap_or_default()
                                )),
                            ])),
                        });
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

            // Call view() and diff surfaces
            match bridge.view(lua) {
                Ok(new_defs) => {
                    // Update existing surfaces
                    for surface in surfaces.iter_mut() {
                        if let Some(def) = new_defs.iter().find(|d| d.name == surface.def.name) {
                            if surface.def.root != def.root {
                                surface.def = def.clone();
                                surface.mark_dirty();
                            }
                        }
                    }

                    // Create new popup surfaces
                    for def in &new_defs {
                        if def.kind == pulpkit_layout::SurfaceKind::Popup {
                            if !surfaces.iter().any(|s| s.name() == def.name) {
                                match crate::runtime::create_popup_surface(def, client) {
                                    Ok(managed) => {
                                        log::info!("Created popup: {}", def.name);
                                        surfaces.push(managed);
                                    }
                                    Err(e) => log::error!("Failed to create popup {}: {e}", def.name),
                                }
                            }
                        }
                    }

                    // Remove popup surfaces no longer in view
                    surfaces.retain(|s| {
                        if s.def.kind == pulpkit_layout::SurfaceKind::Popup {
                            let keep = new_defs.iter().any(|d| d.name == s.def.name);
                            if !keep {
                                log::info!("Destroyed popup: {}", s.def.name);
                            }
                            keep
                        } else {
                            true // keep all windows
                        }
                    });
                }
                Err(e) => {
                    log::error!("Lua view() error: {e}");
                }
            }
        }

        // 8. Render dirty surfaces (gated on frame_ready)
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

/// Map XKB keysyms to human-readable names for Lua.
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

