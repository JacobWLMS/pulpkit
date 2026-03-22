//! Scope — tracks reactive primitives created within its closure and cleans
//! them up on dispose.

use crate::runtime::{ReactiveRuntime, SubscriberId};

/// Tracks all reactive subscribers (effects, computeds) created within its
/// closure. On `dispose()`, unregisters them from the runtime.
pub struct Scope {
    subscriber_ids: Vec<SubscriberId>,
}

impl Scope {
    /// Create a new scope. All subscribers created during `f` are tracked.
    pub fn new<F>(f: F) -> Self
    where
        F: FnOnce(),
    {
        ReactiveRuntime::start_scope();
        f();
        let subscriber_ids = ReactiveRuntime::end_scope();
        Scope { subscriber_ids }
    }

    /// Dispose of this scope: unregister all tracked subscribers from the
    /// runtime and remove them from any signals' subscriber lists.
    pub fn dispose(&self) {
        ReactiveRuntime::dispose_subscribers(&self.subscriber_ids);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Effect, Signal};
    use std::cell::Cell;
    use std::rc::Rc;

    #[test]
    fn scope_disposes_effects() {
        let rt = ReactiveRuntime::new();
        rt.enter(|| {
            let external = Signal::new(0);
            let observed = Rc::new(Cell::new(0i32));
            let scope = Scope::new(|| {
                Effect::new({
                    let external = external.clone();
                    let observed = observed.clone();
                    move || {
                        observed.set(external.get());
                    }
                });
            });
            assert_eq!(observed.get(), 0);
            external.set(5);
            rt.flush();
            assert_eq!(observed.get(), 5); // effect runs
            scope.dispose();
            external.set(99);
            rt.flush();
            assert_eq!(observed.get(), 5); // effect no longer runs -- cleaned up
        });
    }

    #[test]
    fn scope_disposes_multiple_effects() {
        let rt = ReactiveRuntime::new();
        rt.enter(|| {
            let sig = Signal::new(0);
            let obs1 = Rc::new(Cell::new(0i32));
            let obs2 = Rc::new(Cell::new(0i32));
            let scope = Scope::new(|| {
                Effect::new({
                    let sig = sig.clone();
                    let obs1 = obs1.clone();
                    move || { obs1.set(sig.get()); }
                });
                Effect::new({
                    let sig = sig.clone();
                    let obs2 = obs2.clone();
                    move || { obs2.set(sig.get() * 10); }
                });
            });
            sig.set(3);
            rt.flush();
            assert_eq!(obs1.get(), 3);
            assert_eq!(obs2.get(), 30);
            scope.dispose();
            sig.set(9);
            rt.flush();
            // Neither effect should run after dispose
            assert_eq!(obs1.get(), 3);
            assert_eq!(obs2.get(), 30);
        });
    }

    #[test]
    fn effects_outside_scope_unaffected() {
        let rt = ReactiveRuntime::new();
        rt.enter(|| {
            let sig = Signal::new(0);
            let outside_obs = Rc::new(Cell::new(0i32));
            let inside_obs = Rc::new(Cell::new(0i32));

            // Effect created outside the scope
            Effect::new({
                let sig = sig.clone();
                let outside_obs = outside_obs.clone();
                move || { outside_obs.set(sig.get()); }
            });

            let scope = Scope::new(|| {
                Effect::new({
                    let sig = sig.clone();
                    let inside_obs = inside_obs.clone();
                    move || { inside_obs.set(sig.get()); }
                });
            });

            sig.set(5);
            rt.flush();
            assert_eq!(outside_obs.get(), 5);
            assert_eq!(inside_obs.get(), 5);

            scope.dispose();
            sig.set(10);
            rt.flush();
            assert_eq!(outside_obs.get(), 10); // still runs
            assert_eq!(inside_obs.get(), 5);   // disposed, doesn't run
        });
    }
}
