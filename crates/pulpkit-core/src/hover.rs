//! Rust-level hover state — tracks which node the pointer is over.

use pulpkit_layout::damage::DamageRect;
use pulpkit_layout::flex::{LayoutResult, hit_test};

/// Update hover state based on pointer position. Returns damage rects for
/// the old and new hovered nodes (if changed).
pub fn update_hover(
    layout: &LayoutResult,
    pointer_x: f64,
    pointer_y: f64,
    current_hovered: Option<usize>,
) -> (Option<usize>, Vec<DamageRect>) {
    let new_hovered = hit_test(layout, pointer_x as f32, pointer_y as f32);

    if new_hovered == current_hovered {
        return (current_hovered, vec![]);
    }

    let mut damage = Vec::new();

    // Damage the old hovered node
    if let Some(old_idx) = current_hovered {
        if let Some(node) = layout.nodes.get(old_idx) {
            damage.push(node.to_damage_rect());
        }
    }

    // Damage the new hovered node
    if let Some(new_idx) = new_hovered {
        if let Some(node) = layout.nodes.get(new_idx) {
            damage.push(node.to_damage_rect());
        }
    }

    (new_hovered, damage)
}
