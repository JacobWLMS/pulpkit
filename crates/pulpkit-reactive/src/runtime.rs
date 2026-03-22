//! Thread-local reactive runtime that tracks dependency relationships.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

pub type SignalId = u64;
pub type SubscriberId = u64;

/// Trait implemented by Computed and Effect so the runtime can notify them
/// when their dependencies change.
pub trait Subscriber: 'static {
    /// Called synchronously when a dependency changes.
    /// - Computed: marks self dirty, propagates to own subscribers.
    /// - Effect: queues self for re-execution at next flush().
    fn notify(&self);
}

thread_local! {
    static RUNTIME: RefCell<Option<ReactiveRuntimeInner>> = RefCell::new(None);
}

struct ReactiveRuntimeInner {
    tracking_stack: Vec<SubscriberId>,
    next_id: u64,
    /// Registry mapping subscriber IDs to their Subscriber impl.
    subscribers: HashMap<SubscriberId, Rc<dyn Subscriber>>,
    /// Effect IDs queued for re-execution at next flush().
    pending_effects: Vec<SubscriberId>,
    /// Effect runner closures — stored separately so flush() can re-run them
    /// in a tracking context.
    effect_runners: HashMap<SubscriberId, Rc<dyn Fn()>>,
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
                next_id: 0,
                subscribers: HashMap::new(),
                pending_effects: Vec::new(),
                effect_runners: HashMap::new(),
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

    /// Push an existing subscriber ID onto the tracking stack.
    /// Used by Computed and Effect which already have an allocated ID.
    pub fn start_tracking_with_id(id: SubscriberId) {
        RUNTIME.with(|rt| {
            let mut borrow = rt.borrow_mut();
            let inner = borrow
                .as_mut()
                .expect("ReactiveRuntime::start_tracking_with_id called outside of rt.enter()");
            inner.tracking_stack.push(id);
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

    /// Register a subscriber in the runtime's registry.
    pub fn register_subscriber(id: SubscriberId, subscriber: Rc<dyn Subscriber>) {
        RUNTIME.with(|rt| {
            let mut borrow = rt.borrow_mut();
            let inner = borrow
                .as_mut()
                .expect("ReactiveRuntime::register_subscriber called outside of rt.enter()");
            inner.subscribers.insert(id, subscriber);
        })
    }

    /// Register an effect runner closure (called during flush to re-run effects).
    pub fn register_effect_runner(id: SubscriberId, runner: Rc<dyn Fn()>) {
        RUNTIME.with(|rt| {
            let mut borrow = rt.borrow_mut();
            let inner = borrow
                .as_mut()
                .expect("ReactiveRuntime::register_effect_runner called outside of rt.enter()");
            inner.effect_runners.insert(id, runner);
        })
    }

    /// Notify subscribers that a dependency has changed.
    ///
    /// This looks up each subscriber in the registry and calls `notify()`:
    /// - Computed subscribers are marked dirty synchronously.
    /// - Effect subscribers queue themselves for re-execution.
    pub fn notify_subscribers(subscribers: &[SubscriberId]) {
        // Collect the Rc<dyn Subscriber> handles while briefly borrowing the runtime.
        let to_notify: Vec<Rc<dyn Subscriber>> = RUNTIME.with(|rt| {
            let borrow = rt.borrow();
            let inner = borrow
                .as_ref()
                .expect("ReactiveRuntime::notify_subscribers called outside of rt.enter()");
            subscribers
                .iter()
                .filter_map(|id| inner.subscribers.get(id).cloned())
                .collect()
        });

        // Call notify() outside of the runtime borrow to avoid RefCell conflicts.
        for sub in to_notify {
            sub.notify();
        }
    }

    /// Queue an effect for re-execution at the next flush().
    pub fn queue_effect(id: SubscriberId) {
        RUNTIME.with(|rt| {
            let mut borrow = rt.borrow_mut();
            let inner = borrow
                .as_mut()
                .expect("ReactiveRuntime::queue_effect called outside of rt.enter()");
            inner.pending_effects.push(id);
        })
    }

    /// Process all pending effect re-executions, deduplicating by ID.
    pub fn flush(&self) {
        loop {
            // Drain pending effects, deduplicating.
            let effect_ids: Vec<SubscriberId> = RUNTIME.with(|rt| {
                let mut borrow = rt.borrow_mut();
                let inner = borrow
                    .as_mut()
                    .expect("ReactiveRuntime::flush called outside of rt.enter()");
                let mut ids = std::mem::take(&mut inner.pending_effects);
                // Deduplicate while preserving order.
                let mut seen = std::collections::HashSet::new();
                ids.retain(|id| seen.insert(*id));
                ids
            });

            if effect_ids.is_empty() {
                break;
            }

            for id in effect_ids {
                // Look up the effect runner.
                let runner: Option<Rc<dyn Fn()>> = RUNTIME.with(|rt| {
                    let borrow = rt.borrow();
                    let inner = borrow.as_ref().unwrap();
                    inner.effect_runners.get(&id).cloned()
                });

                if let Some(runner) = runner {
                    // Re-run the effect in a tracking context so it re-subscribes.
                    ReactiveRuntime::start_tracking_with_id(id);
                    runner();
                    ReactiveRuntime::stop_tracking();
                }
            }
        }
    }
}

impl Default for ReactiveRuntime {
    fn default() -> Self {
        Self::new()
    }
}
