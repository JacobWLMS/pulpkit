//! Widget node tree — the logical structure before layout.

use std::cell::RefCell;
use std::rc::Rc;

use pulpkit_render::Color;

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

/// State for a [`Slider`](Node::Slider) node.
///
/// The `value` is shared via `Rc<RefCell<f64>>` so that both the paint pass
/// and event handlers can read/write the current position.
pub struct SliderState {
    pub value: Rc<RefCell<f64>>,
    pub min: f64,
    pub max: f64,
    pub on_change: Option<Rc<dyn Fn(f64)>>,
    pub accent_color: Option<Color>,
}

impl Clone for SliderState {
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
            min: self.min,
            max: self.max,
            on_change: self.on_change.clone(),
            accent_color: self.accent_color,
        }
    }
}

/// State for a [`Toggle`](Node::Toggle) node.
///
/// The `checked` boolean is shared via `Rc<RefCell<bool>>` so that both the
/// paint pass and event handlers can read/write the current state.
pub struct ToggleState {
    pub checked: Rc<RefCell<bool>>,
    pub on_change: Option<Rc<dyn Fn(bool)>>,
    pub accent_color: Option<Color>,
}

impl Clone for ToggleState {
    fn clone(&self) -> Self {
        Self {
            checked: self.checked.clone(),
            on_change: self.on_change.clone(),
            accent_color: self.accent_color,
        }
    }
}

/// Reactive or static text content.
///
/// When `Dynamic`, the function is called each time the widget tree is rendered,
/// producing the current text. This enables reactive text that updates when
/// signals change (e.g., a clock display).
#[derive(Clone)]
pub enum TextContent {
    Static(String),
    Dynamic(Rc<dyn Fn() -> String>),
}

impl TextContent {
    /// Resolve the current text value.
    pub fn resolve(&self) -> String {
        match self {
            TextContent::Static(s) => s.clone(),
            TextContent::Dynamic(f) => f(),
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
    /// Text leaf node. Content can be static or dynamic (reactive).
    Text {
        style: StyleProps,
        content: TextContent,
    },
    /// Spacer (flex-grow: 1, takes remaining space).
    Spacer,
    /// Interactive button: a container with event handlers.
    Button {
        style: StyleProps,
        children: Vec<Node>,
        handlers: ButtonHandlers,
    },
    /// Slider: a draggable track for numeric values.
    Slider {
        style: StyleProps,
        state: SliderState,
    },
    /// Toggle switch: a pill-shaped on/off switch.
    Toggle {
        style: StyleProps,
        state: ToggleState,
    },
}

/// Layout direction for a container.
#[derive(Debug, Clone, Copy)]
pub enum Direction {
    Row,
    Column,
}
