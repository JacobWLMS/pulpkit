//! Paint pipeline — walks layout results and issues Skia draw calls.

use std::path::Path;

use crate::flex::LayoutResult;
use crate::tree::{InteractiveKind, Node};
use pulpkit_render::{Canvas, Color, load_image};

/// Paint the laid-out widget tree onto the canvas.
pub fn paint_tree(
    canvas: &mut Canvas,
    layout: &LayoutResult,
    font_family: &str,
) {
    // Track scroll container clip/restore points.
    // Stack of (end_index) — when we reach end_index, restore canvas state.
    let mut scroll_restore_stack: Vec<usize> = Vec::new();

    for (i, layout_node) in layout.nodes.iter().enumerate() {
        // Check if we need to restore from a scroll container.
        while scroll_restore_stack.last().is_some_and(|&end| i >= end) {
            canvas.restore();
            scroll_restore_stack.pop();
        }

        match &layout_node.source_node {
            Node::ScrollContainer { style, scroll_offset, children, .. } => {
                let resolved = style.resolve();
                if let Some(bg) = resolved.bg_color {
                    canvas.draw_rounded_rect(
                        layout_node.x, layout_node.y,
                        layout_node.width, layout_node.height,
                        resolved.border_radius, bg,
                    );
                }
                // Clip to container bounds and translate by scroll offset.
                let offset = scroll_offset.get().as_f64() as f32;
                canvas.save();
                canvas.clip_rect(
                    layout_node.x, layout_node.y,
                    layout_node.width, layout_node.height,
                );
                canvas.translate(0.0, -offset);

                // Calculate end index: self + all descendants.
                let child_count: usize = children.iter()
                    .map(|c| 1 + crate::flex::count_descendants_pub(c))
                    .sum();
                scroll_restore_stack.push(i + 1 + child_count);
            }
            Node::Container { style, .. } => {
                let resolved = style.resolve();
                if let Some(bg) = resolved.bg_color {
                    canvas.draw_rounded_rect(
                        layout_node.x,
                        layout_node.y,
                        layout_node.width,
                        layout_node.height,
                        resolved.border_radius,
                        bg,
                    );
                }
            }
            Node::Text { style, content } => {
                let resolved = style.resolve();
                let color = resolved.text_color.unwrap_or_default();
                let font_size = resolved.font_size.unwrap_or(14.0);
                let resolved_text = content.resolve();
                canvas.draw_text(
                    &resolved_text,
                    layout_node.x,
                    layout_node.y,
                    font_size,
                    font_family,
                    color,
                );
            }
            Node::Interactive { style, kind, .. } => {
                match kind {
                    InteractiveKind::Button { .. } => {
                        // Button draws like a container: background if present.
                        let resolved = style.resolve();
                        if let Some(bg) = resolved.bg_color {
                            canvas.draw_rounded_rect(
                                layout_node.x,
                                layout_node.y,
                                layout_node.width,
                                layout_node.height,
                                resolved.border_radius,
                                bg,
                            );
                        }
                    }
                    InteractiveKind::Input { text, placeholder, .. } => {
                        let resolved = style.resolve();
                        if let Some(bg) = resolved.bg_color {
                            canvas.draw_rounded_rect(
                                layout_node.x, layout_node.y,
                                layout_node.width, layout_node.height,
                                resolved.border_radius, bg,
                            );
                        }
                        let val = text.get();
                        let display = match &val {
                            pulpkit_reactive::DynValue::Str(s) if !s.is_empty() => s.as_str(),
                            _ => placeholder.as_str(),
                        };
                        let color = match &val {
                            pulpkit_reactive::DynValue::Str(s) if !s.is_empty() => {
                                resolved.text_color.unwrap_or_default()
                            }
                            _ => Color::from_hex("#8a929a").unwrap_or_default(), // muted for placeholder
                        };
                        let font_size = resolved.font_size.unwrap_or(14.0);
                        canvas.draw_text(
                            display,
                            layout_node.x + resolved.padding_left,
                            layout_node.y + resolved.padding_top,
                            font_size,
                            font_family,
                            color,
                        );
                    }
                    InteractiveKind::Slider {
                        value,
                        min,
                        max,
                        accent_color,
                        ..
                    } => {
                        let track_height = 6.0_f32;
                        let track_y =
                            layout_node.y + (layout_node.height - track_height) / 2.0;
                        let track_radius = track_height / 2.0;

                        // Track background
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
                        let val = value.get().as_f64();
                        let range = max - min;
                        let ratio = if range > 0.0 {
                            ((val - min) / range).clamp(0.0, 1.0) as f32
                        } else {
                            0.0
                        };
                        let fill_width = layout_node.width * ratio;
                        if fill_width > 0.0 {
                            let accent = accent_color
                                .unwrap_or_else(|| {
                                    Color::from_hex("#8cb4d8").unwrap_or_default()
                                });
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
                    InteractiveKind::Toggle {
                        checked,
                        accent_color,
                        ..
                    } => {
                        let is_checked = checked.get().as_bool();
                        let w = 40.0_f32;
                        let h = 22.0_f32;
                        let padding = 2.0_f32;
                        let circle_r = (h - padding * 2.0) / 2.0;

                        // Track
                        let track_color = if is_checked {
                            accent_color
                                .unwrap_or_else(|| {
                                    Color::from_hex("#8cb4d8").unwrap_or_default()
                                })
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
                        let circle_x = if is_checked {
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
                }
            }
            Node::Image { path, .. } => {
                if let Some(image) = load_image(Path::new(path)) {
                    canvas.draw_image(
                        layout_node.x,
                        layout_node.y,
                        layout_node.width,
                        layout_node.height,
                        &image,
                    );
                }
            }
            Node::DynamicList { style, .. } => {
                // Paint like a container — background if present.
                let resolved = style.resolve();
                if let Some(bg) = resolved.bg_color {
                    canvas.draw_rounded_rect(
                        layout_node.x,
                        layout_node.y,
                        layout_node.width,
                        layout_node.height,
                        resolved.border_radius,
                        bg,
                    );
                }
                // Children are already in the flat layout list and will be painted
                // by subsequent iterations.
            }
            Node::Spacer => {} // nothing to paint
        }
    }

    // Restore any remaining scroll clips.
    while scroll_restore_stack.pop().is_some() {
        canvas.restore();
    }
}
