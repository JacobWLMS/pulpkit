//! Tree diffing — compares two Element trees and produces a list of changes.

use crate::element::{Element, KeyedChild};

/// Result of diffing two element trees.
#[derive(Debug, PartialEq)]
pub enum DiffResult {
    /// Trees are identical.
    Same,
    /// Trees differ.
    Changed(Vec<DiffChange>),
}

/// A single change detected during tree diffing.
#[derive(Debug, PartialEq)]
pub enum DiffChange {
    /// Node at path was replaced with a different type.
    Replace { path: Vec<usize> },
    /// Node's props changed (same type).
    PropsChanged { path: Vec<usize> },
    /// Child was added at index in parent's child list.
    ChildAdded { path: Vec<usize>, index: usize },
    /// Child was removed at index in parent's child list.
    ChildRemoved { path: Vec<usize>, index: usize },
}

/// Diff two lists of surface root elements.
pub fn diff_trees(old: &[Element], new: &[Element]) -> DiffResult {
    let mut changes = Vec::new();
    diff_children(old, new, &[], &mut changes);
    if changes.is_empty() {
        DiffResult::Same
    } else {
        DiffResult::Changed(changes)
    }
}

/// Diff a single element pair.
fn diff_element(old: &Element, new: &Element, path: &[usize], changes: &mut Vec<DiffChange>) {
    // Type mismatch = full replace
    if old.type_tag() != new.type_tag() {
        changes.push(DiffChange::Replace { path: path.to_vec() });
        return;
    }

    // Same type — compare props
    if !props_equal(old, new) {
        changes.push(DiffChange::PropsChanged { path: path.to_vec() });
    }

    // Compare children based on element type
    match (old, new) {
        (Element::Each { children: old_kids, .. }, Element::Each { children: new_kids, .. }) => {
            diff_keyed_children(old_kids, new_kids, path, changes);
        }
        _ => {
            diff_children(old.children(), new.children(), path, changes);
        }
    }
}

/// Diff unkeyed child lists by position.
fn diff_children(
    old: &[Element],
    new: &[Element],
    parent_path: &[usize],
    changes: &mut Vec<DiffChange>,
) {
    let min_len = old.len().min(new.len());

    // Diff shared positions
    for i in 0..min_len {
        let mut child_path = parent_path.to_vec();
        child_path.push(i);
        diff_element(&old[i], &new[i], &child_path, changes);
    }

    // New children added
    for i in min_len..new.len() {
        changes.push(DiffChange::ChildAdded {
            path: parent_path.to_vec(),
            index: i,
        });
    }

    // Old children removed
    for i in min_len..old.len() {
        changes.push(DiffChange::ChildRemoved {
            path: parent_path.to_vec(),
            index: i,
        });
    }
}

/// Diff keyed child lists using key matching.
fn diff_keyed_children(
    old: &[KeyedChild],
    new: &[KeyedChild],
    parent_path: &[usize],
    changes: &mut Vec<DiffChange>,
) {
    use std::collections::HashMap;

    let old_keys: HashMap<&str, usize> = old.iter().enumerate().map(|(i, kc)| (kc.key.as_str(), i)).collect();

    let mut matched_old = vec![false; old.len()];

    for (new_idx, new_kc) in new.iter().enumerate() {
        if let Some(&old_idx) = old_keys.get(new_kc.key.as_str()) {
            matched_old[old_idx] = true;
            // Same key — diff the element
            let mut child_path = parent_path.to_vec();
            child_path.push(new_idx);
            diff_element(&old[old_idx].element, &new_kc.element, &child_path, changes);
        } else {
            // Key not in old — added
            changes.push(DiffChange::ChildAdded {
                path: parent_path.to_vec(),
                index: new_idx,
            });
        }
    }

    // Keys in old but not in new — removed
    for (old_idx, matched) in matched_old.iter().enumerate() {
        if !matched {
            changes.push(DiffChange::ChildRemoved {
                path: parent_path.to_vec(),
                index: old_idx,
            });
        }
    }
}

/// Compare element props (not children) for equality.
fn props_equal(a: &Element, b: &Element) -> bool {
    match (a, b) {
        (
            Element::Container { style: s1, hover_style: h1, direction: d1, .. },
            Element::Container { style: s2, hover_style: h2, direction: d2, .. },
        ) => s1 == s2 && h1 == h2 && d1 == d2,

        (
            Element::Text { style: s1, content: c1 },
            Element::Text { style: s2, content: c2 },
        ) => s1 == s2 && c1 == c2,

        (
            Element::Image { style: s1, path: p1, width: w1, height: h1 },
            Element::Image { style: s2, path: p2, width: w2, height: h2 },
        ) => s1 == s2 && p1 == p2 && w1 == w2 && h1 == h2,

        (Element::Spacer, Element::Spacer) => true,

        (
            Element::Button { style: s1, hover_style: h1, on_click: c1, on_hover: oh1, on_hover_lost: ohl1, .. },
            Element::Button { style: s2, hover_style: h2, on_click: c2, on_hover: oh2, on_hover_lost: ohl2, .. },
        ) => s1 == s2 && h1 == h2 && c1 == c2 && oh1 == oh2 && ohl1 == ohl2,

        (
            Element::Slider { style: s1, value: v1, min: mn1, max: mx1, on_change: c1, accent_color: a1 },
            Element::Slider { style: s2, value: v2, min: mn2, max: mx2, on_change: c2, accent_color: a2 },
        ) => s1 == s2 && v1 == v2 && mn1 == mn2 && mx1 == mx2 && c1 == c2 && a1 == a2,

        (
            Element::Toggle { style: s1, checked: c1, on_toggle: t1, accent_color: a1 },
            Element::Toggle { style: s2, checked: c2, on_toggle: t2, accent_color: a2 },
        ) => s1 == s2 && c1 == c2 && t1 == t2 && a1 == a2,

        (
            Element::Input { style: s1, value: v1, placeholder: p1, on_input: i1 },
            Element::Input { style: s2, value: v2, placeholder: p2, on_input: i2 },
        ) => s1 == s2 && v1 == v2 && p1 == p2 && i1 == i2,

        (
            Element::Scroll { style: s1, scroll_offset: o1, .. },
            Element::Scroll { style: s2, scroll_offset: o2, .. },
        ) => s1 == s2 && o1 == o2,

        (
            Element::Each { style: s1, direction: d1, .. },
            Element::Each { style: s2, direction: d2, .. },
        ) => s1 == s2 && d1 == d2,

        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::element::Direction;
    use crate::style::StyleProps;

    fn text(s: &str) -> Element {
        Element::Text { style: StyleProps::default(), content: s.into() }
    }

    fn container(children: Vec<Element>) -> Element {
        Element::Container {
            style: StyleProps::default(),
            hover_style: None,
            direction: Direction::Row,
            children,
        }
    }

    #[test]
    fn identical_trees_are_same() {
        let a = vec![text("hello")];
        let b = vec![text("hello")];
        assert_eq!(diff_trees(&a, &b), DiffResult::Same);
    }

    #[test]
    fn text_content_changed() {
        let a = vec![text("hello")];
        let b = vec![text("world")];
        match diff_trees(&a, &b) {
            DiffResult::Changed(changes) => {
                assert_eq!(changes.len(), 1);
                assert_eq!(changes[0], DiffChange::PropsChanged { path: vec![0] });
            }
            _ => panic!("expected Changed"),
        }
    }

    #[test]
    fn child_added() {
        let a = vec![container(vec![text("a")])];
        let b = vec![container(vec![text("a"), text("b")])];
        match diff_trees(&a, &b) {
            DiffResult::Changed(changes) => {
                assert!(changes.iter().any(|c| matches!(c, DiffChange::ChildAdded { index: 1, .. })));
            }
            _ => panic!("expected Changed"),
        }
    }

    #[test]
    fn child_removed() {
        let a = vec![container(vec![text("a"), text("b")])];
        let b = vec![container(vec![text("a")])];
        match diff_trees(&a, &b) {
            DiffResult::Changed(changes) => {
                assert!(changes.iter().any(|c| matches!(c, DiffChange::ChildRemoved { index: 1, .. })));
            }
            _ => panic!("expected Changed"),
        }
    }

    #[test]
    fn type_mismatch_is_replace() {
        let a = vec![text("hello")];
        let b = vec![Element::Spacer];
        match diff_trees(&a, &b) {
            DiffResult::Changed(changes) => {
                assert_eq!(changes[0], DiffChange::Replace { path: vec![0] });
            }
            _ => panic!("expected Changed"),
        }
    }

    #[test]
    fn keyed_child_added() {
        let a = vec![Element::Each {
            style: StyleProps::default(),
            direction: Direction::Row,
            children: vec![
                KeyedChild { key: "1".into(), element: text("one") },
            ],
        }];
        let b = vec![Element::Each {
            style: StyleProps::default(),
            direction: Direction::Row,
            children: vec![
                KeyedChild { key: "1".into(), element: text("one") },
                KeyedChild { key: "2".into(), element: text("two") },
            ],
        }];
        match diff_trees(&a, &b) {
            DiffResult::Changed(changes) => {
                assert!(changes.iter().any(|c| matches!(c, DiffChange::ChildAdded { index: 1, .. })));
            }
            _ => panic!("expected Changed"),
        }
    }

    #[test]
    fn keyed_child_removed() {
        let a = vec![Element::Each {
            style: StyleProps::default(),
            direction: Direction::Row,
            children: vec![
                KeyedChild { key: "1".into(), element: text("one") },
                KeyedChild { key: "2".into(), element: text("two") },
            ],
        }];
        let b = vec![Element::Each {
            style: StyleProps::default(),
            direction: Direction::Row,
            children: vec![
                KeyedChild { key: "1".into(), element: text("one") },
            ],
        }];
        match diff_trees(&a, &b) {
            DiffResult::Changed(changes) => {
                assert!(changes.iter().any(|c| matches!(c, DiffChange::ChildRemoved { index: 1, .. })));
            }
            _ => panic!("expected Changed"),
        }
    }

    #[test]
    fn deeply_nested_change() {
        let a = vec![container(vec![container(vec![text("deep")])])];
        let b = vec![container(vec![container(vec![text("changed")])])];
        match diff_trees(&a, &b) {
            DiffResult::Changed(changes) => {
                assert_eq!(changes.len(), 1);
                assert_eq!(changes[0], DiffChange::PropsChanged { path: vec![0, 0, 0] });
            }
            _ => panic!("expected Changed"),
        }
    }
}
