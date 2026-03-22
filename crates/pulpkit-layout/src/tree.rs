//! Widget node tree — the logical structure before layout.

use std::rc::Rc;

use pulpkit_reactive::Signal;
use pulpkit_render::Color;

use crate::style::StyleProps;

/// A rendered property — either a static value or a reactive function.
#[derive(Clone)]
pub enum Prop<T> {
    Static(T),
    Reactive(Rc<dyn Fn() -> T>),
}

impl<T: Clone> Prop<T> {
    pub fn resolve(&self) -> T {
        match self {
            Prop::Static(v) => v.clone(),
            Prop::Reactive(f) => f(),
        }
    }

    pub fn is_reactive(&self) -> bool {
        matches!(self, Prop::Reactive(_))
    }
}

/// Layout direction.
#[derive(Debug, Clone, Copy)]
pub enum Direction {
    Row,
    Column,
}

/// A node in the widget tree.
#[derive(Clone)]
pub enum Node {
    /// Container: row or column of children.
    Container {
        style: Prop<StyleProps>,
        direction: Direction,
        children: Vec<Node>,
    },
    /// Text leaf node. Content and style can be static or reactive.
    Text {
        style: Prop<StyleProps>,
        content: Prop<String>,
    },
    /// Spacer (flex-grow: 1, takes remaining space).
    Spacer,
    /// Interactive widget: button, slider, or toggle.
    Interactive {
        style: Prop<StyleProps>,
        kind: InteractiveKind,
        children: Vec<Node>,
    },
}

/// The kind of interactive widget.
#[derive(Clone)]
pub enum InteractiveKind {
    Button {
        handlers: EventHandlers,
    },
    Slider {
        value: Signal<f64>,
        min: f64,
        max: f64,
        on_change: Option<Rc<dyn Fn(f64)>>,
        accent_color: Option<Color>,
    },
    Toggle {
        checked: Signal<bool>,
        on_change: Option<Rc<dyn Fn(bool)>>,
        accent_color: Option<Color>,
    },
}

/// Event handlers for interactive widgets.
#[derive(Clone, Default)]
pub struct EventHandlers {
    pub on_click: Option<Rc<dyn Fn()>>,
    pub on_scroll_up: Option<Rc<dyn Fn()>>,
    pub on_scroll_down: Option<Rc<dyn Fn()>>,
    pub on_hover: Option<Rc<dyn Fn()>>,
    pub on_hover_lost: Option<Rc<dyn Fn()>>,
}
