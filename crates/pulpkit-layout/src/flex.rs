//! Taffy flexbox integration — converts the widget tree into layout results.

use taffy::prelude::*;

use crate::style::{AlignItems as PkAlignItems, JustifyContent as PkJustifyContent, SizeValue};
use crate::tree::{Direction, Node};
use pulpkit_render::TextRenderer;

/// The result of a layout computation: a flat list of positioned nodes.
pub struct LayoutResult {
    pub nodes: Vec<LayoutNode>,
}

/// A single node with its computed position and size.
pub struct LayoutNode {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub source_node: Node,
}

/// Context attached to taffy leaf nodes for text measurement.
struct MeasureCtx {
    text: String,
    font_size: f32,
}

/// Compute flexbox layout for a widget tree.
///
/// Returns a flat `Vec<LayoutNode>` in pre-order depth-first traversal order.
pub fn compute_layout(
    root: &Node,
    available_width: f32,
    available_height: f32,
    text_renderer: &TextRenderer,
    font_family: &str,
) -> LayoutResult {
    let mut tree: TaffyTree<MeasureCtx> = TaffyTree::new();

    // Build taffy tree, collecting (taffy NodeId, source Node clone) pairs
    // in pre-order so we can read them back in the same order.
    let mut order: Vec<(taffy::NodeId, Node)> = Vec::new();
    let root_id = build_taffy_node(&mut tree, root, &mut order, text_renderer, font_family);

    // Compute layout
    tree.compute_layout_with_measure(
        root_id,
        Size {
            width: AvailableSpace::Definite(available_width),
            height: AvailableSpace::Definite(available_height),
        },
        |known_dimensions, available_space, _node_id, node_context, _style| {
            measure_text(known_dimensions, available_space, node_context, text_renderer, font_family)
        },
    )
    .expect("taffy layout computation failed");

    // Read back positions — walk the order list and accumulate absolute offsets.
    // taffy gives us positions relative to the parent, so we need to walk the
    // tree to compute absolute positions. We'll do a recursive walk instead.
    let mut nodes = Vec::with_capacity(order.len());
    collect_layout(&tree, root_id, 0.0, 0.0, &mut order, &mut 0, &mut nodes);

    LayoutResult { nodes }
}

/// Recursively build a taffy node from a widget Node.
fn build_taffy_node(
    tree: &mut TaffyTree<MeasureCtx>,
    node: &Node,
    order: &mut Vec<(taffy::NodeId, Node)>,
    text_renderer: &TextRenderer,
    font_family: &str,
) -> taffy::NodeId {
    match node {
        Node::Container {
            style,
            direction,
            children,
        } => {
            let taffy_style = to_taffy_style(style, *direction, false);
            let child_ids: Vec<taffy::NodeId> = children
                .iter()
                .map(|c| build_taffy_node(tree, c, order, text_renderer, font_family))
                .collect();
            let id = tree
                .new_with_children(taffy_style, &child_ids)
                .expect("failed to create taffy container node");
            // Insert at the front of order — but we actually want pre-order,
            // so containers go first. We'll use a different approach: insert
            // container into order *before* children by recording position.
            // Actually, since we called children first (to get child_ids),
            // children are already in `order`. We need to insert the container
            // *before* them. Let's fix this with an index.
            // We'll use a simpler approach: track insertion index.
            let insert_idx = order.len() - children.len()
                - children.iter().map(|c| count_descendants(c)).sum::<usize>();
            order.insert(insert_idx, (id, node.clone()));
            id
        }
        Node::Text { style, content } => {
            let resolved = content.resolve();
            let font_size = style.font_size.unwrap_or(14.0);
            let (tw, th) = text_renderer.measure(&resolved, font_size, font_family);
            let mut taffy_style = to_taffy_style(style, Direction::Row, false);
            // Set text node to its measured intrinsic size
            taffy_style.size = Size {
                width: Dimension::from_length(tw),
                height: Dimension::from_length(th),
            };
            let ctx = MeasureCtx {
                text: resolved,
                font_size,
            };
            let id = tree
                .new_leaf_with_context(taffy_style, ctx)
                .expect("failed to create taffy text leaf");
            order.push((id, node.clone()));
            id
        }
        Node::Button {
            style,
            children,
            ..
        } => {
            // Button is laid out exactly like a Container (row direction by default).
            let taffy_style = to_taffy_style(style, Direction::Row, false);
            let child_ids: Vec<taffy::NodeId> = children
                .iter()
                .map(|c| build_taffy_node(tree, c, order, text_renderer, font_family))
                .collect();
            let id = tree
                .new_with_children(taffy_style, &child_ids)
                .expect("failed to create taffy button node");
            let insert_idx = order.len() - children.len()
                - children.iter().map(|c| count_descendants(c)).sum::<usize>();
            order.insert(insert_idx, (id, node.clone()));
            id
        }
        Node::Slider { style, .. } => {
            // Slider is a leaf node with a fixed clickable height (20px)
            // but flexible width (honors w-full / flex-grow from style).
            let mut taffy_style = to_taffy_style(style, Direction::Row, false);
            // Set a fixed height for the clickable area (track is 6px but
            // the overall hit-area is taller for usability).
            taffy_style.size.height = Dimension::from_length(20.0);
            let id = tree
                .new_leaf(taffy_style)
                .expect("failed to create taffy slider leaf");
            order.push((id, node.clone()));
            id
        }
        Node::Toggle { style, .. } => {
            // Toggle is a leaf node with a fixed pill-switch size: 40x22.
            let mut taffy_style = to_taffy_style(style, Direction::Row, false);
            taffy_style.size = Size {
                width: Dimension::from_length(40.0),
                height: Dimension::from_length(22.0),
            };
            let id = tree
                .new_leaf(taffy_style)
                .expect("failed to create taffy toggle leaf");
            order.push((id, node.clone()));
            id
        }
        Node::Spacer => {
            let style = taffy::Style {
                flex_grow: 1.0,
                ..Default::default()
            };
            let id = tree
                .new_leaf(style)
                .expect("failed to create taffy spacer leaf");
            order.push((id, node.clone()));
            id
        }
    }
}

/// Count all descendant nodes (not including self).
fn count_descendants(node: &Node) -> usize {
    match node {
        Node::Container { children, .. } | Node::Button { children, .. } => {
            children.iter().map(|c| 1 + count_descendants(c)).sum()
        }
        _ => 0,
    }
}

/// Measure function for text leaf nodes.
fn measure_text(
    known_dimensions: Size<Option<f32>>,
    _available_space: Size<AvailableSpace>,
    node_context: Option<&mut MeasureCtx>,
    text_renderer: &TextRenderer,
    font_family: &str,
) -> Size<f32> {
    if let Size {
        width: Some(width),
        height: Some(height),
    } = known_dimensions
    {
        return Size { width, height };
    }

    match node_context {
        Some(ctx) => {
            let (w, h) = text_renderer.measure(&ctx.text, ctx.font_size, font_family);
            Size {
                width: known_dimensions.width.unwrap_or(w),
                height: known_dimensions.height.unwrap_or(h),
            }
        }
        None => Size::ZERO,
    }
}

/// Recursively collect layout results in pre-order, converting relative
/// positions to absolute.
fn collect_layout(
    tree: &TaffyTree<MeasureCtx>,
    node_id: taffy::NodeId,
    parent_x: f32,
    parent_y: f32,
    order: &mut Vec<(taffy::NodeId, Node)>,
    idx: &mut usize,
    out: &mut Vec<LayoutNode>,
) {
    let layout = tree.layout(node_id).expect("failed to read taffy layout");
    let abs_x = parent_x + layout.location.x;
    let abs_y = parent_y + layout.location.y;

    // The node at `order[*idx]` should match this `node_id`.
    debug_assert_eq!(order[*idx].0, node_id);
    let source_node = order[*idx].1.clone();
    *idx += 1;

    out.push(LayoutNode {
        x: abs_x,
        y: abs_y,
        width: layout.size.width,
        height: layout.size.height,
        source_node: source_node.clone(),
    });

    // Recurse into children
    let child_count = tree.child_count(node_id);
    for i in 0..child_count {
        let child_id = tree.child_at_index(node_id, i).expect("child index out of bounds");
        collect_layout(tree, child_id, abs_x, abs_y, order, idx, out);
    }
}

/// Find the deepest (most specific) layout node at position (x, y).
/// Returns the index into LayoutResult.nodes, or None if no node contains the point.
/// Nodes are in pre-order depth-first order, so later nodes are children/deeper.
/// We want the last (deepest) match.
pub fn hit_test(layout: &LayoutResult, x: f32, y: f32) -> Option<usize> {
    let mut result = None;
    for (i, node) in layout.nodes.iter().enumerate() {
        if x >= node.x && x <= node.x + node.width
            && y >= node.y && y <= node.y + node.height
        {
            result = Some(i); // keep updating — last match is deepest
        }
    }
    result
}

/// Convert `StyleProps` to a `taffy::Style`.
fn to_taffy_style(
    props: &crate::style::StyleProps,
    direction: Direction,
    _is_spacer: bool,
) -> taffy::Style {
    taffy::Style {
        display: Display::Flex,
        flex_direction: match direction {
            Direction::Row => FlexDirection::Row,
            Direction::Column => FlexDirection::Column,
        },
        align_items: Some(match props.align_items {
            PkAlignItems::Stretch => AlignItems::Stretch,
            PkAlignItems::Start => AlignItems::FlexStart,
            PkAlignItems::Center => AlignItems::Center,
            PkAlignItems::End => AlignItems::FlexEnd,
        }),
        justify_content: Some(match props.justify_content {
            PkJustifyContent::Start => JustifyContent::FlexStart,
            PkJustifyContent::Center => JustifyContent::Center,
            PkJustifyContent::End => JustifyContent::FlexEnd,
            PkJustifyContent::SpaceBetween => JustifyContent::SpaceBetween,
        }),
        padding: Rect {
            left: length(props.padding_left),
            right: length(props.padding_right),
            top: length(props.padding_top),
            bottom: length(props.padding_bottom),
        },
        margin: Rect {
            left: length(props.margin_left),
            right: length(props.margin_right),
            top: length(props.margin_top),
            bottom: length(props.margin_bottom),
        },
        gap: Size {
            width: length(props.gap),
            height: length(props.gap),
        },
        size: Size {
            width: props
                .width
                .as_ref()
                .map(|w| match w {
                    SizeValue::Fill => Dimension::from_percent(1.0),
                    SizeValue::Px(v) => Dimension::from_length(*v),
                })
                .unwrap_or(Dimension::auto()),
            height: props
                .height
                .as_ref()
                .map(|h| match h {
                    SizeValue::Fill => Dimension::from_percent(1.0),
                    SizeValue::Px(v) => Dimension::from_length(*v),
                })
                .unwrap_or(Dimension::auto()),
        },
        flex_grow: props.flex_grow,
        ..Default::default()
    }
}
