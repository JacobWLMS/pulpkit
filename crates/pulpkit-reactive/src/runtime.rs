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
    /// Whether we are currently inside a `batch()` call.
    batching: bool,
    /// Stack of active scopes. Each entry collects subscriber IDs created
    /// while the scope is active.
    scope_stack: Vec<Vec<SubscriberId>>,
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
                batching: false,
                scope_stack: Vec::new(),
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
    /// If a scope is active, the subscriber ID is also tracked by the scope.
    pub fn register_subscriber(id: SubscriberId, subscriber: Rc<dyn Subscriber>) {
        RUNTIME.with(|rt| {
            let mut borrow = rt.borrow_mut();
            let inner = borrow
                .as_mut()
                .expect("ReactiveRuntime::register_subscriber called outside of rt.enter()");
            inner.subscribers.insert(id, subscriber);
            // Track in the current scope, if any.
            if let Some(scope) = inner.scope_stack.last_mut() {
                scope.push(id);
            }
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

    // ---- Batching ----------------------------------------------------------

    /// Check whether the runtime is currently in a batch.
    pub fn is_batching() -> bool {
        RUNTIME.with(|rt| {
            let borrow = rt.borrow();
            borrow
                .as_ref()
                .map(|inner| inner.batching)
                .unwrap_or(false)
        })
    }

    /// Set the batching flag.
    pub fn set_batching(value: bool) {
        RUNTIME.with(|rt| {
            let mut borrow = rt.borrow_mut();
            let inner = borrow
                .as_mut()
                .expect("ReactiveRuntime::set_batching called outside of rt.enter()");
            inner.batching = value;
        })
    }

    /// Static version of `flush()` that doesn't require `&self`.
    pub fn flush_static() {
        // We create a temporary runtime handle to call flush.
        let rt = ReactiveRuntime;
        rt.flush();
    }

    // ---- Scope -------------------------------------------------------------

    /// Push a new scope onto the scope stack. Subsequent subscriber
    /// registrations will be tracked in this scope.
    pub fn start_scope() {
        RUNTIME.with(|rt| {
            let mut borrow = rt.borrow_mut();
            let inner = borrow
                .as_mut()
                .expect("ReactiveRuntime::start_scope called outside of rt.enter()");
            inner.scope_stack.push(Vec::new());
        })
    }

    /// Pop the current scope and return the subscriber IDs that were
    /// registered during this scope.
    pub fn end_scope() -> Vec<SubscriberId> {
        RUNTIME.with(|rt| {
            let mut borrow = rt.borrow_mut();
            let inner = borrow
                .as_mut()
                .expect("ReactiveRuntime::end_scope called outside of rt.enter()");
            inner
                .scope_stack
                .pop()
                .expect("ReactiveRuntime::end_scope called without matching start_scope")
        })
    }

    /// Remove a set of subscribers from the runtime entirely.
    /// This unregisters them from the subscriber registry and removes their
    /// effect runners, preventing them from being notified or re-executed.
    pub fn dispose_subscribers(ids: &[SubscriberId]) {
        RUNTIME.with(|rt| {
            let mut borrow = rt.borrow_mut();
            let inner = borrow
                .as_mut()
                .expect("ReactiveRuntime::dispose_subscribers called outside of rt.enter()");
            for id in ids {
                inner.subscribers.remove(id);
                inner.effect_runners.remove(id);
            }
            // Also remove from pending_effects to avoid stale re-execution.
            let id_set: std::collections::HashSet<SubscriberId> = ids.iter().copied().collect();
            inner.pending_effects.retain(|id| !id_set.contains(id));
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
