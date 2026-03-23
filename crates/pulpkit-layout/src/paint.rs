//! Paint pipeline — walks the element tree and issues draw calls using layout positions.

use crate::damage::DamageRect;
use crate::element::{Direction, Element, KeyedChild};
use crate::flex::{LayoutNode, LayoutResult};
use pulpkit_render::{Canvas, Color, TextRenderer};

/// Paint the laid-out elements onto the canvas.
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
        for rect in rects {
            canvas.save();
            canvas.clip_rect(rect.x as f32, rect.y as f32, rect.width as f32, rect.height as f32);
            paint_walk(canvas, layout, elements, font_family, text_renderer, hovered_node);
            canvas.restore();
        }
    } else {
        paint_walk(canvas, layout, elements, font_family, text_renderer, hovered_node);
    }
}

fn paint_walk(
    canvas: &mut Canvas,
    layout: &LayoutResult,
    elements: &[Element],
    font_family: &str,
    text_renderer: &TextRenderer,
    hovered_node: Option<usize>,
) {
    let mut idx = 0;
    for element in elements {
        paint_element_recursive(canvas, layout, element, &mut idx, font_family, text_renderer, hovered_node);
    }
}

fn paint_element_recursive(
    canvas: &mut Canvas,
    layout: &LayoutResult,
    element: &Element,
    idx: &mut usize,
    font_family: &str,
    text_renderer: &TextRenderer,
    hovered_node: Option<usize>,
) {
    let Some(node) = layout.nodes.get(*idx) else { return };
    let is_hovered = hovered_node == Some(*idx);
    *idx += 1;

    match element {
        Element::Container { style, hover_style, children, .. } => {
            let active_bg = if is_hovered {
                hover_style.as_ref().and_then(|h| h.bg_color).or(style.bg_color)
            } else {
                style.bg_color
            };
            if let Some(bg) = active_bg {
                canvas.draw_rounded_rect(node.x, node.y, node.width, node.height, style.border_radius, bg);
            }
            for child in children {
                paint_element_recursive(canvas, layout, child, idx, font_family, text_renderer, hovered_node);
            }
        }
        Element::Text { style, content } => {
            let color = style.text_color.unwrap_or(Color::new(200, 200, 200, 255));
            let font_size = style.font_size.unwrap_or(14.0);
            canvas.draw_text(content, node.x + style.padding_left, node.y + style.padding_top, font_size, font_family, color, text_renderer);
        }
        Element::Button { style, hover_style, children, .. } => {
            let active_bg = if is_hovered {
                hover_style.as_ref().and_then(|h| h.bg_color).or(style.bg_color)
            } else {
                style.bg_color
            };
            if let Some(bg) = active_bg {
                canvas.draw_rounded_rect(node.x, node.y, node.width, node.height, style.border_radius, bg);
            }
            for child in children {
                paint_element_recursive(canvas, layout, child, idx, font_family, text_renderer, hovered_node);
            }
        }
        Element::Slider { value, min, max, accent_color, .. } => {
            let track_h = 6.0f32;
            let track_y = node.y + (node.height - track_h) / 2.0;
            let track_r = track_h / 2.0;
            canvas.draw_rounded_rect(node.x, track_y, node.width, track_h, track_r, Color::from_hex("#404850").unwrap_or_default());
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
            let circle_x = if *checked { node.x + w - pad - circle_r * 2.0 } else { node.x + pad };
            canvas.draw_rounded_rect(circle_x, node.y + pad, circle_r * 2.0, circle_r * 2.0, circle_r, Color::new(255, 255, 255, 255));
        }
        Element::Image { path, .. } => {
            if let Some(image) = pulpkit_render::load_image(std::path::Path::new(path)) {
                canvas.draw_image(node.x, node.y, node.width, node.height, &image);
            }
        }
        Element::Input { style, value, placeholder, .. } => {
            if let Some(bg) = style.bg_color {
                canvas.draw_rounded_rect(node.x, node.y, node.width, node.height, style.border_radius, bg);
            }
            let display = if value.is_empty() { placeholder.as_str() } else { value.as_str() };
            let color = if value.is_empty() {
                Color::from_hex("#8a929a").unwrap_or_default()
            } else {
                style.text_color.unwrap_or(Color::new(200, 200, 200, 255))
            };
            let font_size = style.font_size.unwrap_or(14.0);
            canvas.draw_text(display, node.x + style.padding_left, node.y + style.padding_top, font_size, font_family, color, text_renderer);
        }
        Element::Each { children, .. } => {
            let style = element.style();
            if let Some(bg) = style.bg_color {
                canvas.draw_rounded_rect(node.x, node.y, node.width, node.height, style.border_radius, bg);
            }
            for kc in children {
                paint_element_recursive(canvas, layout, &kc.element, idx, font_family, text_renderer, hovered_node);
            }
        }
        Element::Scroll { style, children, scroll_offset } => {
            if let Some(bg) = style.bg_color {
                canvas.draw_rounded_rect(node.x, node.y, node.width, node.height, style.border_radius, bg);
            }
            canvas.save();
            canvas.clip_rect(node.x, node.y, node.width, node.height);
            canvas.translate(0.0, -*scroll_offset);
            for child in children {
                paint_element_recursive(canvas, layout, child, idx, font_family, text_renderer, hovered_node);
            }
            canvas.restore();
        }
        Element::Spacer => {}
    }
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
            children: vec![
                Element::Text {
                    style: StyleProps { text_color: Some(Color::new(255, 255, 255, 255)), ..Default::default() },
                    content: "Hello".into(),
                },
                Element::Spacer,
            ],
        }];
        let layout = crate::flex::compute_layout(&elements, 200.0, 36.0, &renderer, "sans-serif");

        let mut data = vec![0u8; 200 * 36 * 4];
        let mut canvas = Canvas::from_buffer(&mut data, 200, 36).unwrap();
        canvas.clear(Color::new(0, 0, 0, 255));
        paint_tree(&mut canvas, &layout, &elements, "sans-serif", &renderer, None, None);
    }
}
