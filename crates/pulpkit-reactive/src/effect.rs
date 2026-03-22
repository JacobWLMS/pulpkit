//! Effect — a side-effect that re-runs when its dependencies change.

use std::rc::Rc;

use crate::runtime::{ReactiveRuntime, Subscriber, SubscriberId};

/// A side-effect that re-runs when its signal/computed dependencies change.
///
/// On creation, the closure runs immediately in a tracking context.
/// When dependencies change, the effect is queued for re-execution at
/// the next `flush()`.
pub struct Effect {
    _id: SubscriberId,
}

impl Effect {
    /// Create a new effect. The closure runs immediately, subscribing to any
    /// signals/computeds it reads. Future changes to those dependencies will
    /// queue the effect for re-execution on `flush()`.
    pub fn new<F: Fn() + 'static>(f: F) -> Self {
        let id = ReactiveRuntime::next_id();
        let runner: Rc<dyn Fn()> = Rc::new(f);

        // Register as a subscriber (for notify dispatch).
        ReactiveRuntime::register_subscriber(id, Rc::new(EffectSubscriber { id }));

        // Register the runner closure (for flush to re-run).
        ReactiveRuntime::register_effect_runner(id, runner.clone());

        // Run immediately in tracking context.
        ReactiveRuntime::start_tracking_with_id(id);
        runner();
        ReactiveRuntime::stop_tracking();

        Effect { _id: id }
    }
}

/// Subscriber implementation for Effect — queues for re-execution at flush().
struct EffectSubscriber {
    id: SubscriberId,
}

impl Subscriber for EffectSubscriber {
    fn notify(&self) {
        ReactiveRuntime::queue_effect(self.id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Computed, Signal};
    use std::cell::Cell;

    #[test]
    fn effect_runs_on_signal_change() {
        let rt = ReactiveRuntime::new();
        rt.enter(|| {
            let count = Signal::new(0);
            let observed = Rc::new(Cell::new(-1i32));
            Effect::new({
                let count = count.clone();
                let observed = observed.clone();
                move || {
                    observed.set(count.get());
                }
            });
            assert_eq!(observed.get(), 0); // effect runs immediately on creation
            count.set(5);
            rt.flush(); // flush needed: effects are deferred until flush()
            assert_eq!(observed.get(), 5); // effect re-ran
        });
    }

    #[test]
    fn diamond_dependency_fires_once() {
        let rt = ReactiveRuntime::new();
        rt.enter(|| {
            let a = Signal::new(1);
            let b = Computed::new({
                let a = a.clone();
                move || a.get() + 1
            });
            let c = Computed::new({
                let a = a.clone();
                move || a.get() * 2
            });
            let run_count = Rc::new(Cell::new(0u32));
            Effect::new({
                let b = b.clone();
                let c = c.clone();
                let run_count = run_count.clone();
                move || {
                    let _ = b.get() + c.get();
                    run_count.set(run_count.get() + 1);
                }
            });
            assert_eq!(run_count.get(), 1); // initial run
            a.set(2);
            rt.flush();
            assert_eq!(run_count.get(), 2); // should fire only once, not twice
        });
    }
}
