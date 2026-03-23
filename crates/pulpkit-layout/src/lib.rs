//! Pulpkit layout engine — element tree, styling, diffing, layout, paint.

pub mod damage;
pub mod diff;
pub mod element;
pub mod style;
pub mod theme;

pub use damage::{DamageRect, merge_damage};
pub use diff::{DiffResult, DiffChange, diff_trees};
pub use element::*;
pub use style::{StyleProps, SizeValue, FontWeight, AlignItems, JustifyContent, parse, parse_with_hover};
pub use theme::Theme;
