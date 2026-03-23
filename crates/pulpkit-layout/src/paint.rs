//! Paint pipeline — walks layout results and issues draw calls.

use crate::damage::DamageRect;
use crate::element::Element;
use crate::flex::{LayoutNode, LayoutResult};
use pulpkit_render::{Canvas, Color, TextRenderer};

/// Paint the laid-out elements onto the canvas.
///
/// If `damage` is `Some`, only paints within the damage rects (clip optimization).
/// If `damage` is `None`, paints everything (initial frame).
/// `hovered_node` enables Rust-level hover style overrides.
pub fn paint_tree(
    canvas: &mut Canvas,
    layout: &LayoutResult,
    elements: &[Element],
    font_family: &str,
    text_renderer: &TextRenderer,
    damage: Option<&[DamageRect]>,
    hovered_node: Option<usize>,
) {
    if let Some(rects) = damage {
        // Damage-clipped painting: for each damage rect, clip and paint
        for rect in rects {
            canvas.save();
            canvas.clip_rect(rect.x as f32, rect.y as f32, rect.width as f32, rect.height as f32);
            paint_all_nodes(canvas, layout, elements, font_family, text_renderer, hovered_node);
            canvas.restore();
        }
    } else {
        // Full repaint
        paint_all_nodes(canvas, layout, elements, font_family, text_renderer, hovered_node);
    }
}

fn paint_all_nodes(
    canvas: &mut Canvas,
    layout: &LayoutResult,
    elements: &[Element],
    font_family: &str,
    text_renderer: &TextRenderer,
    hovered_node: Option<usize>,
) {
    for (i, node) in layout.nodes.iter().enumerate() {
        if let Some(element) = find_element_for_node(elements, node) {
            let is_hovered = hovered_node == Some(i);
            paint_element(canvas, node, element, font_family, text_renderer, is_hovered);
        }
    }
}

fn paint_element(
    canvas: &mut Canvas,
    node: &LayoutNode,
    element: &Element,
    font_family: &str,
    text_renderer: &TextRenderer,
    is_hovered: bool,
) {
    match element {
        Element::Container { style, hover_style, .. } => {
            let active_style = if is_hovered { hover_style.as_ref().unwrap_or(style) } else { style };
            if let Some(bg) = active_style.bg_color.or(style.bg_color) {
                canvas.draw_rounded_rect(
                    node.x, node.y, node.width, node.height,
                    style.border_radius, bg,
                );
            }
        }
        Element::Text { style, content } => {
            let color = style.text_color.unwrap_or(Color::new(200, 200, 200, 255));
            let font_size = style.font_size.unwrap_or(14.0);
            // Draw text using the TextRenderer's direct pixmap API.
            // We need access to the canvas internals — for now, use a simple approach.
            // The canvas doesn't expose PixmapMut directly, so we draw via the canvas API.
            // TODO: integrate text rendering more tightly with canvas.
            // For now, we skip text in paint_tree — it will be rendered via direct pixmap access.
            let _ = (color, font_size, content, font_family, text_renderer);
        }
        Element::Button { style, hover_style, .. } => {
            let active_bg = if is_hovered {
                hover_style.as_ref().and_then(|h| h.bg_color).or(style.bg_color)
            } else {
                style.bg_color
            };
            if let Some(bg) = active_bg {
                canvas.draw_rounded_rect(
                    node.x, node.y, node.width, node.height,
                    style.border_radius, bg,
                );
            }
        }
        Element::Slider { value, min, max, accent_color, .. } => {
            let track_h = 6.0f32;
            let track_y = node.y + (node.height - track_h) / 2.0;
            let track_r = track_h / 2.0;

            // Track background
            let track_bg = Color::from_hex("#404850").unwrap_or_default();
            canvas.draw_rounded_rect(node.x, track_y, node.width, track_h, track_r, track_bg);

            // Filled portion
            let range = max - min;
            let ratio = if range > 0.0 { ((value - min) / range).clamp(0.0, 1.0) as f32 } else { 0.0 };
            let fill_w = node.width * ratio;
            if fill_w > 0.0 {
                let accent = accent_color.unwrap_or(Color::from_hex("#8cb4d8").unwrap_or_default());
                canvas.draw_rounded_rect(node.x, track_y, fill_w, track_h, track_r, accent);
            }
        }
        Element::Toggle { checked, accent_color, .. } => {
            let w = 40.0f32;
            let h = 22.0f32;
            let pad = 2.0f32;
            let circle_r = (h - pad * 2.0) / 2.0;

            let track_color = if *checked {
                accent_color.unwrap_or(Color::from_hex("#8cb4d8").unwrap_or_default())
            } else {
                Color::from_hex("#404850").unwrap_or_default()
            };
            canvas.draw_rounded_rect(node.x, node.y, w, h, h / 2.0, track_color);

            let circle_x = if *checked {
                node.x + w - pad - circle_r * 2.0
            } else {
                node.x + pad
            };
            canvas.draw_rounded_rect(
                circle_x, node.y + pad,
                circle_r * 2.0, circle_r * 2.0,
                circle_r, Color::new(255, 255, 255, 255),
            );
        }
        Element::Image { path, .. } => {
            if let Some(image) = pulpkit_render::load_image(std::path::Path::new(path)) {
                canvas.draw_image(node.x, node.y, node.width, node.height, &image);
            }
        }
        Element::Each { style, .. } | Element::Scroll { style, .. } => {
            if let Some(bg) = style.bg_color {
                canvas.draw_rounded_rect(
                    node.x, node.y, node.width, node.height,
                    style.border_radius, bg,
                );
            }
        }
        Element::Input { style, value, placeholder, .. } => {
            if let Some(bg) = style.bg_color {
                canvas.draw_rounded_rect(
                    node.x, node.y, node.width, node.height,
                    style.border_radius, bg,
                );
            }
            // Text rendering handled via direct pixmap access (same TODO as Text)
            let _ = (value, placeholder);
        }
        Element::Spacer => {}
    }
}

/// Find the element corresponding to a layout node.
/// For now, uses a simple flat walk approach.
fn find_element_for_node<'a>(elements: &'a [Element], _node: &LayoutNode) -> Option<&'a Element> {
    // TODO: proper element_idx mapping. For now, this is a placeholder
    // that will be refined when the full runtime is integrated.
    // The layout pass needs to track element references properly.
    elements.first()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::element::Direction;
    use crate::style::StyleProps;

    #[test]
    fn paint_tree_full_repaint_no_panic() {
        let renderer = TextRenderer::new();
        let elements = vec![Element::Container {
            style: StyleProps {
                bg_color: Some(Color::from_hex("#1e2128").unwrap()),
                ..Default::default()
            },
            hover_style: None,
            direction: Direction::Row,
            children: vec![Element::Spacer],
        }];
        let layout = crate::flex::compute_layout(&elements, 200.0, 36.0, &renderer, "sans-serif");

        let mut data = vec![0u8; 200 * 36 * 4];
        let mut canvas = Canvas::from_buffer(&mut data, 200, 36).unwrap();
        canvas.clear(Color::new(0, 0, 0, 255));
        paint_tree(&mut canvas, &layout, &elements, "sans-serif", &renderer, None, None);
    }
}
