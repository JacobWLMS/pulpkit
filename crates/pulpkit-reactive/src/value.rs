//! Dynamic value type for reactive signals that cross the Rust-Lua boundary.

/// A dynamic value that can live inside a reactive `Signal<DynValue>`.
///
/// Used by interactive widgets (slider, toggle) so that Lua signals
/// (`Signal<DynValue>`) can be passed directly without type-conversion layers.
#[derive(Debug, Clone, PartialEq)]
pub enum DynValue {
    Nil,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
}

impl DynValue {
    /// Extract an `f64`, coercing from Int if needed. Returns 0.0 for non-numeric variants.
    pub fn as_f64(&self) -> f64 {
        match self {
            DynValue::Float(f) => *f,
            DynValue::Int(i) => *i as f64,
            _ => 0.0,
        }
    }

    /// Extract a `bool`. Returns `false` for non-bool variants.
    pub fn as_bool(&self) -> bool {
        matches!(self, DynValue::Bool(true))
    }
}
