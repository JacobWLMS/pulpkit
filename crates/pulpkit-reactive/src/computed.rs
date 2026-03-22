//! Computed — a lazy derived value that auto-tracks signal dependencies.

use std::cell::RefCell;
use std::rc::Rc;

use crate::runtime::{ReactiveRuntime, Subscriber, SubscriberId};

struct ComputedInner<T> {
    id: SubscriberId,
    compute: Rc<dyn Fn() -> T>,
    value: Option<T>,
    dirty: bool,
    subscribers: Vec<SubscriberId>,
}

/// A lazy derived value that auto-tracks its signal dependencies.
///
/// Cloning a `Computed` produces a handle to the same underlying value.
pub struct Computed<T> {
    inner: Rc<RefCell<ComputedInner<T>>>,
}

impl<T: Clone + 'static> Computed<T> {
    /// Create a new computed value from the given closure.
    /// The closure is NOT evaluated immediately — it runs lazily on the first `get()`.
    pub fn new<F: Fn() -> T + 'static>(compute: F) -> Self {
        let id = ReactiveRuntime::next_id();
        let inner = Rc::new(RefCell::new(ComputedInner {
            id,
            compute: Rc::new(compute),
            value: None,
            dirty: true, // needs initial computation
            subscribers: Vec::new(),
        }));

        // Register self in the runtime's subscriber registry.
        let weak = Rc::downgrade(&inner);
        ReactiveRuntime::register_subscriber(
            id,
            Rc::new(ComputedSubscriber { inner: weak }),
        );

        Computed { inner }
    }

    /// Read the current value. If the value is dirty or not yet computed,
    /// re-evaluate the closure in a tracking context.
    pub fn get(&self) -> T {
        // Check if we need to re-evaluate.
        let needs_eval = {
            let borrow = self.inner.borrow();
            borrow.dirty || borrow.value.is_none()
        };

        if needs_eval {
            // Clone the compute Rc so we can call it without holding a borrow.
            let (id, compute) = {
                let borrow = self.inner.borrow();
                (borrow.id, Rc::clone(&borrow.compute))
            };

            // Enter tracking context so signal reads subscribe this computed.
            ReactiveRuntime::start_tracking_with_id(id);
            let new_value = compute();
            ReactiveRuntime::stop_tracking();

            let mut borrow = self.inner.borrow_mut();
            borrow.value = Some(new_value);
            borrow.dirty = false;
        }

        // Register the current tracker (if any) as a subscriber of this computed.
        if let Some(tracker) = ReactiveRuntime::current_tracker() {
            let mut borrow = self.inner.borrow_mut();
            if !borrow.subscribers.contains(&tracker) {
                borrow.subscribers.push(tracker);
            }
        }

        self.inner.borrow().value.clone().unwrap()
    }

    /// Return this computed's unique ID.
    pub fn id(&self) -> SubscriberId {
        self.inner.borrow().id
    }
}

impl<T> Clone for Computed<T> {
    fn clone(&self) -> Self {
        Computed {
            inner: Rc::clone(&self.inner),
        }
    }
}

/// Subscriber implementation for Computed — marks it dirty and propagates
/// to its own subscribers.
struct ComputedSubscriber<T> {
    inner: std::rc::Weak<RefCell<ComputedInner<T>>>,
}

impl<T: 'static> Subscriber for ComputedSubscriber<T> {
    fn notify(&self) {
        if let Some(inner) = self.inner.upgrade() {
            let mut borrow = inner.borrow_mut();
            borrow.dirty = true;
            // Propagate to our own subscribers (other computeds/effects that
            // depend on this computed).
            let subs = borrow.subscribers.clone();
            drop(borrow);
            if !subs.is_empty() {
                ReactiveRuntime::notify_subscribers(&subs);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Signal;
    use std::cell::Cell;

    #[test]
    fn computed_derives_from_signal() {
        let rt = ReactiveRuntime::new();
        rt.enter(|| {
            let count = Signal::new(5);
            let doubled = Computed::new({
                let count = count.clone();
                move || count.get() * 2
            });
            assert_eq!(doubled.get(), 10);
            count.set(7);
            // No flush needed: computed lazily re-evaluates on get()
            assert_eq!(doubled.get(), 14);
        });
    }

    #[test]
    fn computed_is_lazy() {
        let rt = ReactiveRuntime::new();
        rt.enter(|| {
            let call_count = Rc::new(Cell::new(0u32));
            let count = Signal::new(1);
            let doubled = Computed::new({
                let count = count.clone();
                let call_count = call_count.clone();
                move || {
                    call_count.set(call_count.get() + 1);
                    count.get() * 2
                }
            });
            assert_eq!(call_count.get(), 0); // not evaluated yet
            assert_eq!(doubled.get(), 2);
            assert_eq!(call_count.get(), 1); // evaluated once
            assert_eq!(doubled.get(), 2);
            assert_eq!(call_count.get(), 1); // cached, not re-evaluated
        });
    }
}
