//! Paint pipeline — walks layout results and issues Skia draw calls.

use crate::flex::LayoutResult;
use crate::tree::Node;
use pulpkit_render::Canvas;

/// Paint the laid-out widget tree onto the canvas.
///
/// When `hovered_index` is `Some(i)`, the node at index `i` will use its
/// hover style overrides (e.g. `hover_bg_color` instead of `bg_color`).
pub fn paint_tree(
    canvas: &mut Canvas,
    layout: &LayoutResult,
    font_family: &str,
    hovered_index: Option<usize>,
) {
    for (i, layout_node) in layout.nodes.iter().enumerate() {
        let is_hovered = hovered_index == Some(i);
        match &layout_node.source_node {
            Node::Container { style, .. } | Node::Button { style, .. } => {
                let bg = if is_hovered {
                    style.hover_bg_color.or(style.bg_color)
                } else {
                    style.bg_color
                };
                if let Some(bg) = bg {
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
                let color = if is_hovered {
                    style
                        .hover_text_color
                        .or(style.text_color)
                        .unwrap_or_default()
                } else {
                    style.text_color.unwrap_or_default()
                };
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
            Node::Spacer => {} // nothing to paint
        }
    }
}
