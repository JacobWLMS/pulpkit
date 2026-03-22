//! Widget node tree — the logical structure before layout.

use std::rc::Rc;

use crate::style::StyleProps;

/// Event handlers for a [`Button`](Node::Button) node.
///
/// Callbacks are wrapped in `Rc` so that `ButtonHandlers` (and thus `Node`)
/// can be cheaply cloned without losing the closures.
pub struct ButtonHandlers {
    pub on_click: Option<Rc<dyn Fn()>>,
    pub on_scroll_up: Option<Rc<dyn Fn()>>,
    pub on_scroll_down: Option<Rc<dyn Fn()>>,
}

impl Clone for ButtonHandlers {
    fn clone(&self) -> Self {
        Self {
            on_click: self.on_click.clone(),
            on_scroll_up: self.on_scroll_up.clone(),
            on_scroll_down: self.on_scroll_down.clone(),
        }
    }
}

impl Default for ButtonHandlers {
    fn default() -> Self {
        Self {
            on_click: None,
            on_scroll_up: None,
            on_scroll_down: None,
        }
    }
}

/// A node in the widget tree.
#[derive(Clone)]
pub enum Node {
    /// Container: row or column of children.
    Container {
        style: StyleProps,
        direction: Direction,
        children: Vec<Node>,
    },
    /// Text leaf node.
    Text {
        style: StyleProps,
        content: String,
    },
    /// Spacer (flex-grow: 1, takes remaining space).
    Spacer,
    /// Interactive button: a container with event handlers.
    Button {
        style: StyleProps,
        children: Vec<Node>,
        handlers: ButtonHandlers,
    },
}

/// Layout direction for a container.
#[derive(Debug, Clone, Copy)]
pub enum Direction {
    Row,
    Column,
}
