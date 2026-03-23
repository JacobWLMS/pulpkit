//! Pulpkit subscription system — manages calloop event sources for
//! timers, subprocess streams, exec, IPC, and config watching.

pub mod manager;

pub use manager::{SubHandle, SubMessage, SubscriptionManager};
