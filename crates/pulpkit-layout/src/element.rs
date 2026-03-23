//! Element tree — the v3 widget tree. Plain data, no signals or closures.

use crate::style::StyleProps;
use pulpkit_render::Color;

/// Index into a flat element/layout node list.
pub type NodeId = usize;

/// Layout direction for containers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Row,
    Column,
}

/// A message value — inert data produced by user interactions or subscriptions.
#[derive(Debug, Clone, PartialEq)]
pub struct Message {
    pub msg_type: String,
    pub data: Option<MessageData>,
}

/// Payload data attached to a message.
#[derive(Debug, Clone, PartialEq)]
pub enum MessageData {
    String(String),
    Float(f64),
    Bool(bool),
    Int(i64),
    Table(Vec<(String, MessageData)>),
}

impl MessageData {
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            MessageData::Float(f) => Some(*f),
            MessageData::Int(i) => Some(*i as f64),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            MessageData::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            MessageData::Bool(b) => Some(*b),
            _ => None,
        }
    }
}

/// A keyed child element (used inside `Each` lists).
#[derive(Debug, Clone, PartialEq)]
pub struct KeyedChild {
    pub key: String,
    pub element: Element,
}

/// A widget element in the view tree. All data is owned — no signals, no closures.
#[derive(Debug, Clone, PartialEq)]
pub enum Element {
    /// Row or column container.
    Container {
        style: StyleProps,
        hover_style: Option<StyleProps>,
        direction: Direction,
        children: Vec<Element>,
    },
    /// Text leaf.
    Text {
        style: StyleProps,
        content: String,
    },
    /// Image from file.
    Image {
        style: StyleProps,
        path: String,
        width: f32,
        height: f32,
    },
    /// Flexible spacer (flex-grow: 1).
    Spacer,
    /// Clickable button container.
    Button {
        style: StyleProps,
        hover_style: Option<StyleProps>,
        on_click: Option<Message>,
        on_hover: Option<Message>,
        on_hover_lost: Option<Message>,
        children: Vec<Element>,
    },
    /// Draggable slider.
    Slider {
        style: StyleProps,
        value: f64,
        min: f64,
        max: f64,
        on_change: Option<Message>,
        accent_color: Option<Color>,
    },
    /// On/off toggle.
    Toggle {
        style: StyleProps,
        checked: bool,
        on_toggle: Option<Message>,
        accent_color: Option<Color>,
    },
    /// Text input field.
    Input {
        style: StyleProps,
        value: String,
        placeholder: String,
        on_input: Option<Message>,
    },
    /// Scrollable container.
    Scroll {
        style: StyleProps,
        children: Vec<Element>,
        scroll_offset: f32,
    },
    /// Keyed list (for efficient diffing).
    Each {
        style: StyleProps,
        direction: Direction,
        children: Vec<KeyedChild>,
    },
}

impl Element {
    /// Get the style props for this element.
    pub fn style(&self) -> &StyleProps {
        match self {
            Element::Container { style, .. }
            | Element::Text { style, .. }
            | Element::Image { style, .. }
            | Element::Button { style, .. }
            | Element::Slider { style, .. }
            | Element::Toggle { style, .. }
            | Element::Input { style, .. }
            | Element::Scroll { style, .. }
            | Element::Each { style, .. } => style,
            Element::Spacer => &StyleProps::EMPTY,
        }
    }

    /// Get children if this element has them.
    pub fn children(&self) -> &[Element] {
        match self {
            Element::Container { children, .. }
            | Element::Button { children, .. }
            | Element::Scroll { children, .. } => children,
            Element::Each { children: _, .. } => {
                // Each children are keyed — return empty; use keyed_children() instead.
                &[]
            }
            _ => &[],
        }
    }

    /// Get keyed children (only for Each elements).
    pub fn keyed_children(&self) -> &[KeyedChild] {
        match self {
            Element::Each { children, .. } => children,
            _ => &[],
        }
    }

    /// Return the discriminant name for type comparison in diffing.
    pub fn type_tag(&self) -> &'static str {
        match self {
            Element::Container { .. } => "container",
            Element::Text { .. } => "text",
            Element::Image { .. } => "image",
            Element::Spacer => "spacer",
            Element::Button { .. } => "button",
            Element::Slider { .. } => "slider",
            Element::Toggle { .. } => "toggle",
            Element::Input { .. } => "input",
            Element::Scroll { .. } => "scroll",
            Element::Each { .. } => "each",
        }
    }
}

/// A surface definition returned by Lua view() — represents a window or popup.
#[derive(Debug, Clone)]
pub struct SurfaceDef {
    pub name: String,
    pub kind: SurfaceKind,
    pub anchor: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub exclusive: bool,
    pub monitor: MonitorTarget,
    pub dismiss_on_outside: bool,
    pub root: Element,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceKind {
    Window,
    Popup,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MonitorTarget {
    All,
    Primary,
    Named(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn element_container_with_children() {
        let el = Element::Container {
            style: StyleProps::default(),
            hover_style: None,
            direction: Direction::Row,
            children: vec![
                Element::Text {
                    style: StyleProps::default(),
                    content: "hello".into(),
                },
                Element::Spacer,
            ],
        };
        assert_eq!(el.children().len(), 2);
        assert_eq!(el.type_tag(), "container");
    }

    #[test]
    fn message_equality() {
        let a = Message { msg_type: "click".into(), data: None };
        let b = Message { msg_type: "click".into(), data: None };
        assert_eq!(a, b);

        let c = Message { msg_type: "click".into(), data: Some(MessageData::Int(42)) };
        assert_ne!(a, c);
    }

    #[test]
    fn keyed_child() {
        let kc = KeyedChild {
            key: "ws-1".into(),
            element: Element::Text {
                style: StyleProps::default(),
                content: "1".into(),
            },
        };
        assert_eq!(kc.key, "ws-1");
    }
}
