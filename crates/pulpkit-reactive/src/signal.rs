//! Reactive signal — a value that tracks subscribers and notifies them on change.

use std::cell::RefCell;
use std::rc::Rc;

use crate::runtime::{ReactiveRuntime, SignalId, SubscriberId};

/// A reactive value that tracks subscribers and notifies them on change.
///
/// Cloning a `Signal` produces a handle to the same underlying value.
pub struct Signal<T> {
    inner: Rc<RefCell<SignalInner<T>>>,
}

struct SignalInner<T> {
    id: SignalId,
    value: T,
    subscribers: Vec<SubscriberId>,
}

impl<T: Clone + 'static> Signal<T> {
    /// Create a new signal with the given initial value.
    /// Allocates a unique ID from the current reactive runtime.
    pub fn new(value: T) -> Self {
        let id = ReactiveRuntime::next_id();
        Signal {
            inner: Rc::new(RefCell::new(SignalInner {
                id,
                value,
                subscribers: Vec::new(),
            })),
        }
    }

    /// Read the current value. If there is a current tracker (a computed or
    /// effect being evaluated), this signal registers that tracker as a
    /// subscriber so it will be notified on future changes.
    pub fn get(&self) -> T {
        let inner = self.inner.borrow();
        if let Some(sub_id) = ReactiveRuntime::current_tracker() {
            drop(inner);
            let mut inner = self.inner.borrow_mut();
            if !inner.subscribers.contains(&sub_id) {
                inner.subscribers.push(sub_id);
            }
            inner.value.clone()
        } else {
            inner.value.clone()
        }
    }

    /// Set a new value and notify all subscribers.
    pub fn set(&self, value: T) {
        let mut inner = self.inner.borrow_mut();
        inner.value = value;
        let subscribers: Vec<SubscriberId> = inner.subscribers.clone();
        drop(inner);
        if !subscribers.is_empty() {
            ReactiveRuntime::notify_subscribers(&subscribers);
        }
    }

    /// Return this signal's unique ID.
    pub fn id(&self) -> SignalId {
        self.inner.borrow().id
    }

    /// Check whether the given subscriber is registered on this signal.
    pub fn has_subscriber(&self, id: SubscriberId) -> bool {
        self.inner.borrow().subscribers.contains(&id)
    }

    /// Return the number of subscribers (for testing).
    pub fn subscriber_count(&self) -> usize {
        self.inner.borrow().subscribers.len()
    }
}

impl<T> Clone for Signal<T> {
    fn clone(&self) -> Self {
        Signal {
            inner: Rc::clone(&self.inner),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::ReactiveRuntime;

    #[test]
    fn signal_get_set() {
        let rt = ReactiveRuntime::new();
        rt.enter(|| {
            let s = Signal::new(42i32);
            assert_eq!(s.get(), 42);
            s.set(99);
            assert_eq!(s.get(), 99);
        });
    }

    #[test]
    fn signal_tracks_subscribers() {
        let rt = ReactiveRuntime::new();
        rt.enter(|| {
            let s = Signal::new(10);
            let sub_id = ReactiveRuntime::start_tracking();
            let _ = s.get(); // should register sub_id as subscriber
            ReactiveRuntime::stop_tracking();
            assert!(s.has_subscriber(sub_id));
        });
    }

    #[test]
    fn signal_clone_shares_state() {
        let rt = ReactiveRuntime::new();
        rt.enter(|| {
            let s1 = Signal::new(5);
            let s2 = s1.clone();
            s1.set(10);
            assert_eq!(s2.get(), 10);
        });
    }

    #[test]
    fn signal_notify_on_set() {
        let rt = ReactiveRuntime::new();
        rt.enter(|| {
            let s = Signal::new(0);
            // Simulate a subscriber reading the signal
            let sub_id = ReactiveRuntime::start_tracking();
            let _ = s.get();
            ReactiveRuntime::stop_tracking();
            // Setting should queue the subscriber for notification
            s.set(1);
            // The subscriber should still be tracked
            assert!(s.has_subscriber(sub_id));
        });
    }

    #[test]
    fn signal_no_duplicate_subscribers() {
        let rt = ReactiveRuntime::new();
        rt.enter(|| {
            let s = Signal::new(0);
            let sub_id = ReactiveRuntime::start_tracking();
            let _ = s.get();
            let _ = s.get(); // read twice in same tracking scope
            ReactiveRuntime::stop_tracking();
            // Should only have one entry for this subscriber
            assert!(s.has_subscriber(sub_id));
            assert_eq!(s.subscriber_count(), 1);
        });
    }
}
