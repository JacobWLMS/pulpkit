//! Paint pipeline — walks layout results and issues Skia draw calls.

use crate::animation::AnimationManager;
use crate::flex::LayoutResult;
use crate::tree::Node;
use pulpkit_render::{Canvas, Color};

/// Paint the laid-out widget tree onto the canvas.
///
/// When `hovered_index` is `Some(i)`, the node at index `i` will use its
/// hover style overrides (e.g. `hover_bg_color` instead of `bg_color`).
///
/// Active animations in `animations` take priority over both base and hover
/// colors, providing smooth transitions between states.
pub fn paint_tree(
    canvas: &mut Canvas,
    layout: &LayoutResult,
    font_family: &str,
    hovered_index: Option<usize>,
    animations: &mut AnimationManager,
) {
    for (i, layout_node) in layout.nodes.iter().enumerate() {
        let is_hovered = hovered_index == Some(i);
        match &layout_node.source_node {
            Node::Container { style, .. } | Node::Button { style, .. } => {
                // Check for an active animation first; fall back to hover/base logic.
                let bg = if let Some(animated_color) = animations.get_bg(i) {
                    Some(animated_color)
                } else if is_hovered {
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
            Node::Slider { state, .. } => {
                let track_height = 6.0_f32;
                let track_y = layout_node.y + (layout_node.height - track_height) / 2.0;
                let track_radius = track_height / 2.0;

                // Track background (outline color)
                let track_bg =
                    Color::from_hex("#404850").unwrap_or_default();
                canvas.draw_rounded_rect(
                    layout_node.x,
                    track_y,
                    layout_node.width,
                    track_height,
                    track_radius,
                    track_bg,
                );

                // Filled portion
                let val = *state.value.borrow();
                let range = state.max - state.min;
                let ratio = if range > 0.0 {
                    ((val - state.min) / range).clamp(0.0, 1.0) as f32
                } else {
                    0.0
                };
                let fill_width = layout_node.width * ratio;
                if fill_width > 0.0 {
                    let accent = state
                        .accent_color
                        .unwrap_or_else(|| Color::from_hex("#8cb4d8").unwrap_or_default());
                    canvas.draw_rounded_rect(
                        layout_node.x,
                        track_y,
                        fill_width,
                        track_height,
                        track_radius,
                        accent,
                    );
                }
            }
            Node::Toggle { state, .. } => {
                let checked = *state.checked.borrow();
                let w = 40.0_f32;
                let h = 22.0_f32;
                let padding = 2.0_f32;
                let circle_r = (h - padding * 2.0) / 2.0;

                // Track
                let track_color = if checked {
                    state
                        .accent_color
                        .unwrap_or_else(|| Color::from_hex("#8cb4d8").unwrap_or_default())
                } else {
                    Color::from_hex("#404850").unwrap_or_default()
                };
                canvas.draw_rounded_rect(
                    layout_node.x,
                    layout_node.y,
                    w,
                    h,
                    h / 2.0,
                    track_color,
                );

                // Circle indicator
                let circle_x = if checked {
                    layout_node.x + w - padding - circle_r * 2.0
                } else {
                    layout_node.x + padding
                };
                let circle_y = layout_node.y + padding;
                let circle_color = Color::new(255, 255, 255, 255);
                canvas.draw_rounded_rect(
                    circle_x,
                    circle_y,
                    circle_r * 2.0,
                    circle_r * 2.0,
                    circle_r,
                    circle_color,
                );
            }
            Node::Spacer => {} // nothing to paint
        }
    }
}
