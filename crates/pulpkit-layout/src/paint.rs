//! Paint pipeline — walks layout results and issues Skia draw calls.

use crate::flex::LayoutResult;
use crate::tree::Node;
use pulpkit_render::Canvas;

/// Paint the laid-out widget tree onto the canvas.
pub fn paint_tree(canvas: &mut Canvas, layout: &LayoutResult, font_family: &str) {
    for layout_node in &layout.nodes {
        match &layout_node.source_node {
            Node::Container { style, .. } => {
                if let Some(bg) = style.bg_color {
                    canvas.draw_rounded_rect(
                        layout_node.x,
                        layout_node.y,
                        layout_node.width,
                        layout_node.height,
                        style.border_radius,
                        bg,
                    );
                }
            }
            Node::Text { style, content } => {
                let color = style.text_color.unwrap_or_default();
                let font_size = style.font_size.unwrap_or(14.0);
                canvas.draw_text(
                    content,
                    layout_node.x,
                    layout_node.y,
                    font_size,
                    font_family,
                    color,
                );
            }
            Node::Button { style, .. } => {
                if let Some(bg) = style.bg_color {
                    canvas.draw_rounded_rect(
                        layout_node.x,
                        layout_node.y,
                        layout_node.width,
                        layout_node.height,
                        style.border_radius,
                        bg,
                    );
                }
            }
            Node::Spacer => {} // nothing to paint
        }
    }
}
