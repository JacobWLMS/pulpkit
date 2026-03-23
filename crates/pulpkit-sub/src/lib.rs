//! Pulpkit subscription system — manages calloop event sources for
//! timers, subprocess streams, exec, IPC, and config watching.

pub mod exec;
pub mod manager;
pub mod stream;

pub use manager::{SubHandle, SubMessage, SubscriptionManager};
