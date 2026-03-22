//! Thread-local reactive runtime that tracks dependency relationships.

use std::cell::RefCell;

pub type SignalId = u64;
pub type SubscriberId = u64;

thread_local! {
    static RUNTIME: RefCell<Option<ReactiveRuntimeInner>> = RefCell::new(None);
}

struct ReactiveRuntimeInner {
    tracking_stack: Vec<SubscriberId>,
    pending_notifications: Vec<SubscriberId>,
    next_id: u64,
    batching: bool,
}

/// A thread-local reactive runtime that tracks dependency relationships
/// between signals, computed values, and effects.
pub struct ReactiveRuntime;

impl ReactiveRuntime {
    /// Create a new reactive runtime.
    pub fn new() -> Self {
        ReactiveRuntime
    }

    /// Set this runtime as the thread-local runtime for the duration of `f`.
    pub fn enter<F, R>(&self, f: F) -> R
    where
        F: FnOnce() -> R,
    {
        RUNTIME.with(|rt| {
            let prev = rt.borrow_mut().replace(ReactiveRuntimeInner {
                tracking_stack: Vec::new(),
                pending_notifications: Vec::new(),
                next_id: 0,
                batching: false,
            });
            let result = f();
            *rt.borrow_mut() = prev;
            result
        })
    }

    /// Push a new subscriber onto the tracking stack and return its ID.
    pub fn start_tracking() -> SubscriberId {
        RUNTIME.with(|rt| {
            let mut borrow = rt.borrow_mut();
            let inner = borrow
                .as_mut()
                .expect("ReactiveRuntime::start_tracking called outside of rt.enter()");
            let id = inner.next_id;
            inner.next_id += 1;
            inner.tracking_stack.push(id);
            id
        })
    }

    /// Pop from the tracking stack.
    pub fn stop_tracking() {
        RUNTIME.with(|rt| {
            let mut borrow = rt.borrow_mut();
            let inner = borrow
                .as_mut()
                .expect("ReactiveRuntime::stop_tracking called outside of rt.enter()");
            inner.tracking_stack.pop();
        })
    }

    /// Peek at the top of the tracking stack.
    pub fn current_tracker() -> Option<SubscriberId> {
        RUNTIME.with(|rt| {
            let borrow = rt.borrow();
            let inner = borrow.as_ref()?;
            inner.tracking_stack.last().copied()
        })
    }

    /// Allocate a new monotonic ID.
    pub fn next_id() -> u64 {
        RUNTIME.with(|rt| {
            let mut borrow = rt.borrow_mut();
            let inner = borrow
                .as_mut()
                .expect("ReactiveRuntime::next_id called outside of rt.enter()");
            let id = inner.next_id;
            inner.next_id += 1;
            id
        })
    }

    /// Queue subscribers for notification.
    pub fn notify_subscribers(subscribers: &[SubscriberId]) {
        RUNTIME.with(|rt| {
            let mut borrow = rt.borrow_mut();
            let inner = borrow
                .as_mut()
                .expect("ReactiveRuntime::notify_subscribers called outside of rt.enter()");
            inner.pending_notifications.extend_from_slice(subscribers);
        })
    }
}

impl Default for ReactiveRuntime {
    fn default() -> Self {
        Self::new()
    }
}
