//! Batch — defers effect re-execution during a closure, flushing once at the end.

use crate::runtime::ReactiveRuntime;

/// Execute `f` in a batch context. Multiple signal changes inside the batch
/// only trigger effects once when the batch completes.
///
/// Batches can be nested; only the outermost batch triggers a flush.
pub fn batch<F, R>(f: F) -> R
where
    F: FnOnce() -> R,
{
    let was_batching = ReactiveRuntime::is_batching();
    ReactiveRuntime::set_batching(true);
    let result = f();
    if !was_batching {
        ReactiveRuntime::set_batching(false);
        ReactiveRuntime::flush_static();
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Effect, Signal};
    use std::cell::Cell;
    use std::rc::Rc;

    #[test]
    fn batch_defers_notifications() {
        let rt = ReactiveRuntime::new();
        rt.enter(|| {
            let a = Signal::new(1);
            let b = Signal::new(2);
            let run_count = Rc::new(Cell::new(0u32));
            let sum = Rc::new(Cell::new(0i32));
            Effect::new({
                let a = a.clone();
                let b = b.clone();
                let run_count = run_count.clone();
                let sum = sum.clone();
                move || {
                    sum.set(a.get() + b.get());
                    run_count.set(run_count.get() + 1);
                }
            });
            assert_eq!(run_count.get(), 1); // initial
            batch(|| {
                a.set(10);
                b.set(20);
            });
            // Effect should fire once after batch, not twice
            assert_eq!(run_count.get(), 2);
            assert_eq!(sum.get(), 30);
        });
    }

    #[test]
    fn nested_batch_only_flushes_outermost() {
        let rt = ReactiveRuntime::new();
        rt.enter(|| {
            let a = Signal::new(0);
            let run_count = Rc::new(Cell::new(0u32));
            Effect::new({
                let a = a.clone();
                let run_count = run_count.clone();
                move || {
                    let _ = a.get();
                    run_count.set(run_count.get() + 1);
                }
            });
            assert_eq!(run_count.get(), 1);
            batch(|| {
                a.set(1);
                batch(|| {
                    a.set(2);
                });
                // Inner batch should NOT have flushed
                assert_eq!(run_count.get(), 1);
                a.set(3);
            });
            // Only outermost batch flushes
            assert_eq!(run_count.get(), 2);
        });
    }

    #[test]
    fn batch_returns_value() {
        let rt = ReactiveRuntime::new();
        rt.enter(|| {
            let result = batch(|| 42);
            assert_eq!(result, 42);
        });
    }
}
