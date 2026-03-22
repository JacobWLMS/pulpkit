//! Dirty-tracking wiring — connects reactive Props to surface dirty flags.
//!
//! Walks a Node tree and creates Effects for each `Prop::Reactive`.
//! When a reactive property's dependencies change, the Effect sets the
//! surface's dirty flag and wakes the event loop via the calloop channel.

use std::cell::Cell;
use std::rc::Rc;

use calloop::channel::Sender;
use pulpkit_layout::tree::{InteractiveKind, Node, Prop};
use pulpkit_reactive::Effect;

use crate::runtime::RuntimeEvent;
use crate::surfaces::ManagedSurface;

/// Walk all managed surfaces and wire dirty-tracking Effects for reactive Props.
pub fn wire_dirty_tracking(
    surfaces: &[ManagedSurface],
    wake_sender: &Sender<RuntimeEvent>,
) {
    for surface in surfaces {
        wire_node(&surface.root, &surface.dirty, wake_sender);
    }
}

/// Recursively wire a single node's reactive properties.
fn wire_node(
    node: &Node,
    dirty: &Rc<Cell<bool>>,
    wake: &Sender<RuntimeEvent>,
) {
    match node {
        Node::Text { content, style, .. } => {
            wire_prop_string(content, dirty, wake);
            wire_prop_style(style, dirty, wake);
        }
        Node::Container {
            children, style, ..
        } => {
            wire_prop_style(style, dirty, wake);
            for child in children {
                wire_node(child, dirty, wake);
            }
        }
        Node::Interactive {
            style,
            kind,
            children,
            ..
        } => {
            wire_prop_style(style, dirty, wake);
            match kind {
                InteractiveKind::Slider { value, .. } => {
                    let v = value.clone();
                    let d = dirty.clone();
                    let w = wake.clone();
                    // Read the signal inside an Effect so its dependencies are tracked.
                    std::mem::forget(Effect::new(move || {
                        let _ = v.get();
                        d.set(true);
                        let _ = w.send(RuntimeEvent::Redraw);
                    }));
                }
                InteractiveKind::Toggle { checked, .. } => {
                    let v = checked.clone();
                    let d = dirty.clone();
                    let w = wake.clone();
                    std::mem::forget(Effect::new(move || {
                        let _ = v.get();
                        d.set(true);
                        let _ = w.send(RuntimeEvent::Redraw);
                    }));
                }
                InteractiveKind::Button { .. } => {}
            }
            for child in children {
                wire_node(child, dirty, wake);
            }
        }
        Node::Spacer => {}
    }
}

/// If a Prop<String> is reactive, wire an Effect that marks dirty on change.
fn wire_prop_string(
    prop: &Prop<String>,
    dirty: &Rc<Cell<bool>>,
    wake: &Sender<RuntimeEvent>,
) {
    if prop.is_reactive() {
        let f = prop.clone();
        let d = dirty.clone();
        let w = wake.clone();
        std::mem::forget(Effect::new(move || {
            let _ = f.resolve();
            d.set(true);
            let _ = w.send(RuntimeEvent::Redraw);
        }));
    }
}

/// If a Prop<StyleProps> is reactive, wire an Effect that marks dirty on change.
fn wire_prop_style(
    prop: &Prop<pulpkit_layout::StyleProps>,
    dirty: &Rc<Cell<bool>>,
    wake: &Sender<RuntimeEvent>,
) {
    if prop.is_reactive() {
        let f = prop.clone();
        let d = dirty.clone();
        let w = wake.clone();
        std::mem::forget(Effect::new(move || {
            let _ = f.resolve();
            d.set(true);
            let _ = w.send(RuntimeEvent::Redraw);
        }));
    }
}
