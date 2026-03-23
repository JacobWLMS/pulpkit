//! Taffy-based flexbox layout engine for the Element tree.

use taffy::prelude::*;

use crate::damage::DamageRect;
use crate::element::{Direction, Element};
use crate::style::{AlignItems, JustifyContent, SizeValue, StyleProps};
use pulpkit_render::TextRenderer;

/// Result of layout computation — a flat list of positioned nodes with elements.
#[derive(Debug)]
pub struct LayoutResult {
    pub nodes: Vec<LayoutNode>,
    /// Flat element list parallel to nodes — each node's corresponding element.
    pub elements: Vec<Element>,
}

/// A positioned node in the layout result.
#[derive(Debug)]
pub struct LayoutNode {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    /// Index into the element list this layout node corresponds to.
    pub element_idx: usize,
}

impl LayoutNode {
    /// Convert to a damage rect.
    pub fn to_damage_rect(&self) -> DamageRect {
        DamageRect::new(
            self.x as i32,
            self.y as i32,
            self.width.ceil() as i32,
            self.height.ceil() as i32,
        )
    }
}

/// Compute layout for an element tree within the given bounds.
pub fn compute_layout(
    elements: &[Element],
    width: f32,
    height: f32,
    text_renderer: &TextRenderer,
    font_family: &str,
) -> LayoutResult {
    let mut tree = TaffyTree::new();
    let mut nodes = Vec::new();
    let mut flat_elements: Vec<usize> = Vec::new();

    // Build taffy nodes for each root element
    let mut taffy_roots = Vec::new();
    for (i, element) in elements.iter().enumerate() {
        let node = build_taffy_node(element, &mut tree, &mut flat_elements, elements, i, text_renderer, font_family);
        taffy_roots.push(node);
    }

    // Create a root container that holds all elements
    let root_style = Style {
        size: Size {
            width: Dimension::length(width),
            height: Dimension::length(height),
        },
        flex_direction: FlexDirection::Row,
        align_items: Some(taffy::AlignItems::Stretch),
        ..Default::default()
    };
    let root = tree.new_with_children(root_style, &taffy_roots).unwrap();

    // Compute layout
    let available = Size {
        width: AvailableSpace::Definite(width),
        height: AvailableSpace::Definite(height),
    };
    tree.compute_layout(root, available).unwrap();

    // Extract positions by walking the tree
    collect_layout(&tree, root, 0.0, 0.0, &flat_elements, &mut nodes, true);

    // Build flat element list in same order as layout nodes
    let mut flat_els = Vec::new();
    for element in elements {
        flatten_element(element, &mut flat_els);
    }

    LayoutResult { nodes, elements: flat_els }
}

fn build_taffy_node(
    element: &Element,
    tree: &mut TaffyTree,
    flat: &mut Vec<usize>,
    all_elements: &[Element],
    elem_idx: usize,
    text_renderer: &TextRenderer,
    font_family: &str,
) -> NodeId {
    let _idx = flat.len();
    flat.push(elem_idx);

    match element {
        Element::Container { style, direction, children, .. } => {
            let child_nodes: Vec<NodeId> = children.iter().enumerate()
                .map(|(i, child)| build_taffy_node(child, tree, flat, all_elements, elem_idx * 1000 + i, text_renderer, font_family))
                .collect();
            let taffy_style = to_taffy_style(style, *direction);
            tree.new_with_children(taffy_style, &child_nodes).unwrap()
        }
        Element::Button { style, children, .. } => {
            let child_nodes: Vec<NodeId> = children.iter().enumerate()
                .map(|(i, child)| build_taffy_node(child, tree, flat, all_elements, elem_idx * 1000 + i, text_renderer, font_family))
                .collect();
            let taffy_style = to_taffy_style(style, Direction::Row);
            tree.new_with_children(taffy_style, &child_nodes).unwrap()
        }
        Element::Text { style, content } => {
            let font_size = style.font_size.unwrap_or(14.0);
            let (tw, th) = text_renderer.measure_text(content, font_family, font_size);
            let mut s = to_taffy_style(style, Direction::Row);
            s.size.width = Dimension::length(tw + style.padding_left + style.padding_right);
            s.size.height = Dimension::length(th + style.padding_top + style.padding_bottom);
            tree.new_leaf(s).unwrap()
        }
        Element::Image { style, width, height, .. } => {
            let mut s = to_taffy_style(style, Direction::Row);
            s.size.width = Dimension::length(*width);
            s.size.height = Dimension::length(*height);
            tree.new_leaf(s).unwrap()
        }
        Element::Spacer => {
            let s = Style {
                flex_grow: 1.0,
                ..Default::default()
            };
            tree.new_leaf(s).unwrap()
        }
        Element::Slider { style, .. } => {
            let mut s = to_taffy_style(style, Direction::Row);
            if s.size.height == Dimension::auto() {
                s.size.height = Dimension::length(24.0);
            }
            tree.new_leaf(s).unwrap()
        }
        Element::Toggle { style, .. } => {
            let mut s = to_taffy_style(style, Direction::Row);
            s.size.width = Dimension::length(40.0);
            s.size.height = Dimension::length(22.0);
            tree.new_leaf(s).unwrap()
        }
        Element::Input { style, .. } => {
            let mut s = to_taffy_style(style, Direction::Row);
            if s.size.height == Dimension::auto() {
                s.size.height = Dimension::length(32.0);
            }
            tree.new_leaf(s).unwrap()
        }
        Element::Scroll { style, children, .. } => {
            let child_nodes: Vec<NodeId> = children.iter().enumerate()
                .map(|(i, child)| build_taffy_node(child, tree, flat, all_elements, elem_idx * 1000 + i, text_renderer, font_family))
                .collect();
            let mut s = to_taffy_style(style, Direction::Column);
            s.overflow.y = taffy::Overflow::Hidden;
            tree.new_with_children(s, &child_nodes).unwrap()
        }
        Element::Each { style, direction, children } => {
            let child_nodes: Vec<NodeId> = children.iter().enumerate()
                .map(|(i, kc)| build_taffy_node(&kc.element, tree, flat, all_elements, elem_idx * 1000 + i, text_renderer, font_family))
                .collect();
            let taffy_style = to_taffy_style(style, *direction);
            tree.new_with_children(taffy_style, &child_nodes).unwrap()
        }
    }
}

fn to_taffy_style(props: &StyleProps, direction: Direction) -> Style {
    Style {
        flex_direction: match direction {
            Direction::Row => FlexDirection::Row,
            Direction::Column => FlexDirection::Column,
        },
        size: Size {
            width: match props.width {
                Some(SizeValue::Px(v)) => Dimension::length(v),
                Some(SizeValue::Fill) => Dimension::percent(1.0),
                None => Dimension::auto(),
            },
            height: match props.height {
                Some(SizeValue::Px(v)) => Dimension::length(v),
                Some(SizeValue::Fill) => Dimension::percent(1.0),
                None => Dimension::auto(),
            },
        },
        min_size: Size {
            width: match props.min_width {
                Some(SizeValue::Px(v)) => Dimension::length(v),
                _ => Dimension::auto(),
            },
            height: Dimension::auto(),
        },
        max_size: Size {
            width: match props.max_width {
                Some(SizeValue::Px(v)) => Dimension::length(v),
                _ => Dimension::auto(),
            },
            height: Dimension::auto(),
        },
        padding: Rect {
            top: LengthPercentage::length(props.padding_top),
            right: LengthPercentage::length(props.padding_right),
            bottom: LengthPercentage::length(props.padding_bottom),
            left: LengthPercentage::length(props.padding_left),
        },
        margin: Rect {
            top: LengthPercentageAuto::length(props.margin_top),
            right: LengthPercentageAuto::length(props.margin_right),
            bottom: LengthPercentageAuto::length(props.margin_bottom),
            left: LengthPercentageAuto::length(props.margin_left),
        },
        gap: Size {
            width: LengthPercentage::length(props.gap),
            height: LengthPercentage::length(props.gap),
        },
        flex_grow: props.flex_grow,
        align_items: Some(match props.align_items {
            AlignItems::Stretch => taffy::AlignItems::Stretch,
            AlignItems::Start => taffy::AlignItems::FlexStart,
            AlignItems::Center => taffy::AlignItems::Center,
            AlignItems::End => taffy::AlignItems::FlexEnd,
        }),
        justify_content: Some(match props.justify_content {
            JustifyContent::Start => taffy::JustifyContent::FlexStart,
            JustifyContent::Center => taffy::JustifyContent::Center,
            JustifyContent::End => taffy::JustifyContent::FlexEnd,
            JustifyContent::SpaceBetween => taffy::JustifyContent::SpaceBetween,
        }),
        ..Default::default()
    }
}

fn collect_layout(
    tree: &TaffyTree,
    node: NodeId,
    parent_x: f32,
    parent_y: f32,
    flat: &[usize],
    out: &mut Vec<LayoutNode>,
    is_root: bool,
) {
    let layout = tree.layout(node).unwrap();
    let x = parent_x + layout.location.x;
    let y = parent_y + layout.location.y;

    if !is_root {
        let idx = out.len(); // This isn't quite right for the flat index mapping
        out.push(LayoutNode {
            x,
            y,
            width: layout.size.width,
            height: layout.size.height,
            element_idx: if idx < flat.len() { flat[idx] } else { 0 },
        });
    }

    for &child in tree.children(node).unwrap().iter() {
        collect_layout(tree, child, x, y, flat, out, false);
    }
}

/// Hit-test: find the deepest layout node at (x, y).
pub fn hit_test(layout: &LayoutResult, x: f32, y: f32) -> Option<usize> {
    let mut best = None;
    for (i, node) in layout.nodes.iter().enumerate() {
        if x >= node.x && x < node.x + node.width && y >= node.y && y < node.y + node.height {
            best = Some(i);
        }
    }
    best
}

/// Hit-test and return the element at (x, y), if any.
pub fn hit_test_element(layout: &LayoutResult, x: f32, y: f32) -> Option<&Element> {
    let idx = hit_test(layout, x, y)?;
    layout.elements.get(idx)
}

/// Flatten an element tree into a pre-order list (same order as paint/layout walk).
fn flatten_element(element: &Element, out: &mut Vec<Element>) {
    out.push(element.clone());
    match element {
        Element::Container { children, .. }
        | Element::Button { children, .. }
        | Element::Scroll { children, .. } => {
            for child in children {
                flatten_element(child, out);
            }
        }
        Element::Each { children, .. } => {
            for kc in children {
                flatten_element(&kc.element, out);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::StyleProps;

    #[test]
    fn layout_row_with_spacer() {
        let renderer = TextRenderer::new();
        let elements = vec![
            Element::Container {
                style: StyleProps {
                    width: Some(SizeValue::Px(200.0)),
                    height: Some(SizeValue::Px(36.0)),
                    ..Default::default()
                },
                hover_style: None,
                direction: Direction::Row,
                children: vec![
                    Element::Text {
                        style: StyleProps::default(),
                        content: "Hello".into(),
                    },
                    Element::Spacer,
                    Element::Text {
                        style: StyleProps::default(),
                        content: "World".into(),
                    },
                ],
            },
        ];
        let result = compute_layout(&elements, 200.0, 36.0, &renderer, "sans-serif");
        assert!(!result.nodes.is_empty(), "layout should produce nodes");
        // Last text node should be pushed to the right
        let last = result.nodes.last().unwrap();
        assert!(last.x > 50.0, "spacer should push last child right, got x={}", last.x);
    }

    #[test]
    fn hit_test_finds_node() {
        let layout = LayoutResult {
            nodes: vec![
                LayoutNode { x: 0.0, y: 0.0, width: 200.0, height: 36.0, element_idx: 0 },
                LayoutNode { x: 10.0, y: 5.0, width: 50.0, height: 26.0, element_idx: 1 },
            ],
            elements: vec![Element::Spacer, Element::Spacer],
        };
        assert_eq!(hit_test(&layout, 20.0, 10.0), Some(1));
        assert_eq!(hit_test(&layout, 150.0, 10.0), Some(0));
        assert_eq!(hit_test(&layout, 300.0, 10.0), None);
    }
}
