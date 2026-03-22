//! Widget node tree — the logical structure before layout.

use crate::style::StyleProps;

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
}

/// Layout direction for a container.
#[derive(Debug, Clone, Copy)]
pub enum Direction {
    Row,
    Column,
}
