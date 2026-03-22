//! Pulpkit reactive engine — fine-grained reactivity inspired by SolidJS.

pub mod runtime;
pub mod signal;

pub use runtime::ReactiveRuntime;
pub use signal::Signal;
