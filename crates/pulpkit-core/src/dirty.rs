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
///
/// Each Effect reads its reactive dependency (to register as a subscriber),
/// then sets the surface dirty flag and wakes the event loop. The wake is
/// only sent when the flag transitions from clean to dirty to avoid flooding
/// the calloop channel.
pub fn wire_dirty_tracking(
    surfaces: &[ManagedSurface],
    wake_sender: &Sender<RuntimeEvent>,
) {
    for surface in surfaces {
        wire_node(&surface.root, &surface.dirty, wake_sender);
    }
    // Clear dirty flags set by the initial Effect runs — surfaces were
    // already rendered during setup so they start clean.
    for surface in surfaces {
        surface.dirty.set(false);
    }
}

fn wire_node(node: &Node, dirty: &Rc<Cell<bool>>, wake: &Sender<RuntimeEvent>) {
    match node {
        Node::Image { style, .. } => {
            wire_style_prop(style, dirty, wake);
        }
        Node::Text { content, style, .. } => {
            wire_prop(content, dirty, wake);
            wire_style_prop(style, dirty, wake);
        }
        Node::Container {
            children, style, ..
        } => {
            wire_style_prop(style, dirty, wake);
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
            wire_style_prop(style, dirty, wake);
            match kind {
                InteractiveKind::Slider { value, .. } => {
                    wire_signal(value, dirty, wake);
                }
                InteractiveKind::Toggle { checked, .. } => {
                    wire_signal(checked, dirty, wake);
                }
                InteractiveKind::Button { .. } => {}
            }
            for child in children {
                wire_node(child, dirty, wake);
            }
        }
        Node::DynamicList {
            style,
            resolve,
            cached_children,
            ..
        } => {
            wire_style_prop(style, dirty, wake);
            // Wire the resolve function itself — when the items signal changes,
            // resolve() will read different data, so we need to mark dirty.
            let r = resolve.clone();
            let d = dirty.clone();
            let w = wake.clone();
            std::mem::forget(Effect::new(move || {
                let _ = r(); // track signal dependencies inside items_fn
                mark_dirty(&d, &w);
            }));
            // Also wire any reactive props inside the initially cached children.
            for child in cached_children.borrow().iter() {
                wire_node(child, dirty, wake);
            }
        }
        Node::Spacer => {}
    }
}

/// Wire a reactive Prop<String> (e.g. text content).
fn wire_prop(prop: &Prop<String>, dirty: &Rc<Cell<bool>>, wake: &Sender<RuntimeEvent>) {
    if !prop.is_reactive() {
        return;
    }
    let f = prop.clone();
    let d = dirty.clone();
    let w = wake.clone();
    std::mem::forget(Effect::new(move || {
        let _ = f.resolve(); // track signal dependencies
        mark_dirty(&d, &w);
    }));
}

/// Wire a reactive Prop<StyleProps>.
fn wire_style_prop(
    prop: &Prop<pulpkit_layout::StyleProps>,
    dirty: &Rc<Cell<bool>>,
    wake: &Sender<RuntimeEvent>,
) {
    if !prop.is_reactive() {
        return;
    }
    let f = prop.clone();
    let d = dirty.clone();
    let w = wake.clone();
    std::mem::forget(Effect::new(move || {
        let _ = f.resolve(); // track signal dependencies
        mark_dirty(&d, &w);
    }));
}

/// Wire a Signal<DynValue> (slider value, toggle checked).
fn wire_signal(
    signal: &pulpkit_reactive::Signal<pulpkit_reactive::DynValue>,
    dirty: &Rc<Cell<bool>>,
    wake: &Sender<RuntimeEvent>,
) {
    let s = signal.clone();
    let d = dirty.clone();
    let w = wake.clone();
    std::mem::forget(Effect::new(move || {
        let _ = s.get(); // track signal dependencies
        mark_dirty(&d, &w);
    }));
}

/// Set the dirty flag and wake the event loop — but only if the flag was
/// previously clean, to avoid flooding the calloop channel with redundant
/// Redraw messages.
fn mark_dirty(dirty: &Rc<Cell<bool>>, wake: &Sender<RuntimeEvent>) {
    if !dirty.get() {
        dirty.set(true);
        let _ = wake.send(RuntimeEvent::Redraw);
    }
}
