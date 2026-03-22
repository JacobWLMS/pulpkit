//! Unified event dispatch — click, hover, and scroll handling.

use pulpkit_layout::tree::{InteractiveKind, Node};
use pulpkit_layout::{hit_test, LayoutResult};
use pulpkit_reactive::DynValue;
use wayland_client::backend::ObjectId;

use crate::popups::ManagedPopup;
use crate::surfaces::ManagedSurface;

/// Result of a click dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClickResult {
    /// A handler was found and invoked.
    Handled,
    /// No interactive widget was hit.
    Miss,
}

/// Dispatch a pointer button press to the correct surface/popup.
///
/// Finds the target surface by ObjectId, hit-tests against its layout,
/// and invokes the appropriate handler (button click, slider set, toggle flip).
pub fn dispatch_click(
    surfaces: &[ManagedSurface],
    popups: &[ManagedPopup],
    x: f64,
    y: f64,
    surface_id: &ObjectId,
) -> ClickResult {
    let fx = x as f32;
    let fy = y as f32;

    // Check popups first (they overlay surfaces).
    for popup in popups {
        if popup.surface_id().as_ref() == Some(surface_id) {
            if let Some(layout) = popup.layout() {
                return dispatch_click_on_layout(layout, fx, fy);
            }
        }
    }

    // Then check bar surfaces.
    for surface in surfaces {
        if surface.surface.surface_id() == *surface_id {
            if let Some(ref layout) = surface.layout {
                return dispatch_click_on_layout(layout, fx, fy);
            }
        }
    }

    ClickResult::Miss
}

/// Hit-test a layout and dispatch click to the deepest interactive node.
fn dispatch_click_on_layout(layout: &LayoutResult, x: f32, y: f32) -> ClickResult {
    // Walk nodes in reverse (deepest first) to find interactive targets.
    for node in layout.nodes.iter().rev() {
        if x < node.x || x > node.x + node.width || y < node.y || y > node.y + node.height {
            continue;
        }

        match &node.source_node {
            Node::Interactive { kind, .. } => match kind {
                InteractiveKind::Button { handlers } => {
                    if let Some(cb) = &handlers.on_click {
                        cb();
                        return ClickResult::Handled;
                    }
                }
                InteractiveKind::Slider {
                    value,
                    min,
                    max,
                    on_change,
                    ..
                } => {
                    let ratio = ((x - node.x) / node.width).clamp(0.0, 1.0) as f64;
                    let new_val = min + (max - min) * ratio;
                    value.set(DynValue::Float(new_val));
                    if let Some(cb) = on_change {
                        cb(new_val);
                    }
                    return ClickResult::Handled;
                }
                InteractiveKind::Toggle {
                    checked, on_change, ..
                } => {
                    let new_val = !checked.get().as_bool();
                    checked.set(DynValue::Bool(new_val));
                    if let Some(cb) = on_change {
                        cb(new_val);
                    }
                    return ClickResult::Handled;
                }
            },
            _ => {}
        }
    }

    ClickResult::Miss
}

/// Dispatch a scroll event (vertical axis) to the target surface.
///
/// Finds the deepest button with a scroll handler and invokes it.
pub fn dispatch_scroll(
    surfaces: &[ManagedSurface],
    popups: &[ManagedPopup],
    x: f64,
    y: f64,
    surface_id: &ObjectId,
    scroll_up: bool,
) -> bool {
    let fx = x as f32;
    let fy = y as f32;

    // Check popups first.
    for popup in popups {
        if popup.surface_id().as_ref() == Some(surface_id) {
            if let Some(layout) = popup.layout() {
                if let Some(handled) = dispatch_scroll_on_layout(layout, fx, fy, scroll_up) {
                    return handled;
                }
            }
        }
    }

    // Then bar surfaces.
    for surface in surfaces {
        if surface.surface.surface_id() == *surface_id {
            if let Some(ref layout) = surface.layout {
                if let Some(handled) = dispatch_scroll_on_layout(layout, fx, fy, scroll_up) {
                    return handled;
                }
            }
        }
    }

    false
}

/// Try to dispatch a scroll on a specific layout. Returns Some(true) if handled.
fn dispatch_scroll_on_layout(
    layout: &LayoutResult,
    x: f32,
    y: f32,
    scroll_up: bool,
) -> Option<bool> {
    for node in layout.nodes.iter().rev() {
        if x < node.x || x > node.x + node.width || y < node.y || y > node.y + node.height {
            continue;
        }

        if let Node::Interactive {
            kind: InteractiveKind::Button { handlers },
            ..
        } = &node.source_node
        {
            let handler = if scroll_up {
                &handlers.on_scroll_up
            } else {
                &handlers.on_scroll_down
            };
            if let Some(cb) = handler {
                cb();
                return Some(true);
            }
        }
    }
    None
}

/// Dispatch hover events — updates hovered_node, fires on_hover/on_hover_lost.
///
/// Returns true if the hover state changed (surface needs re-render).
pub fn dispatch_hover(
    surfaces: &mut [ManagedSurface],
    x: f64,
    y: f64,
    surface_id: &ObjectId,
) -> bool {
    let fx = x as f32;
    let fy = y as f32;
    let mut changed = false;

    for surface in surfaces.iter_mut() {
        if surface.surface.surface_id() != *surface_id {
            continue;
        }

        if let Some(ref layout) = surface.layout {
            let hit = hit_test(layout, fx, fy);
            if hit != surface.hovered_node {
                // Fire on_hover_lost for the old node.
                if let Some(old_idx) = surface.hovered_node {
                    fire_hover_handler(&layout.nodes[old_idx].source_node, false);
                }
                // Fire on_hover for the new node.
                if let Some(new_idx) = hit {
                    fire_hover_handler(&layout.nodes[new_idx].source_node, true);
                }
                surface.hovered_node = hit;
                surface.mark_dirty();
                changed = true;
            }
        }
        break;
    }

    changed
}

/// Dispatch a pointer-leave event — clears hover state on all surfaces.
pub fn dispatch_leave(surfaces: &mut [ManagedSurface], surface_id: &ObjectId) -> bool {
    let mut changed = false;
    for surface in surfaces.iter_mut() {
        if surface.surface.surface_id() != *surface_id {
            continue;
        }
        if let Some(old_idx) = surface.hovered_node {
            if let Some(ref layout) = surface.layout {
                fire_hover_handler(&layout.nodes[old_idx].source_node, false);
            }
            surface.hovered_node = None;
            surface.mark_dirty();
            changed = true;
        }
        break;
    }
    changed
}

/// Fire on_hover or on_hover_lost for an interactive node.
fn fire_hover_handler(node: &Node, entering: bool) {
    if let Node::Interactive {
        kind: InteractiveKind::Button { handlers },
        ..
    } = node
    {
        if entering {
            if let Some(cb) = &handlers.on_hover {
                cb();
            }
        } else {
            if let Some(cb) = &handlers.on_hover_lost {
                cb();
            }
        }
    }
}
