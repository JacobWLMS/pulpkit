//! Pulpkit layout engine

pub mod theme;
pub mod style;
pub mod tree;
pub mod flex;
pub mod paint;

pub use theme::Theme;
pub use style::{StyleProps, SizeValue, FontWeight, AlignItems, JustifyContent};
pub use tree::{Node, Direction};
pub use flex::{compute_layout, LayoutResult, LayoutNode};
pub use paint::paint_tree;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::{Node, Direction};
    use crate::style::StyleProps;
    use pulpkit_render::TextRenderer;

    #[test]
    fn row_with_spacer_fills_width() {
        let text_renderer = TextRenderer::new();
        let root = Node::Container {
            style: StyleProps {
                width: Some(SizeValue::Px(200.0)),
                height: Some(SizeValue::Px(36.0)),
                ..Default::default()
            },
            direction: Direction::Row,
            children: vec![
                Node::Text {
                    style: StyleProps::default(),
                    content: "Hello".into(),
                },
                Node::Spacer,
                Node::Text {
                    style: StyleProps::default(),
                    content: "World".into(),
                },
            ],
        };
        let result = compute_layout(&root, 200.0, 36.0, &text_renderer, "sans-serif");
        // Last text node should be pushed to the right edge by the spacer
        let last = result.nodes.last().unwrap();
        assert!(last.x > 100.0, "spacer should push last child to right half, got x={}", last.x);
    }
}
