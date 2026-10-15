use moon_async_utils::get_concurrency;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

/// Minimum number of items that each thread must have to process before
/// work is spread across threads. Spawning a thread costs about as much as
/// expanding a handful of nodes, so small inputs (a single project's tasks
/// for example) stay on the calling thread, and the number of threads
/// spawned scales with the input until it reaches the available parallelism.
pub const PARALLEL_MIN_ITEMS_PER_THREAD: usize = 8;

/// Return the number of threads (including the calling thread) that
/// [`map_in_parallel`] will use for the provided number of items. A value of
/// 1 means the items are processed serially on the calling thread.
pub fn parallel_threads_for(count: usize) -> usize {
    (count / PARALLEL_MIN_ITEMS_PER_THREAD).clamp(1, get_concurrency())
}

/// Return true if the provided number of items will be processed across
/// multiple threads by [`map_in_parallel`].
pub fn should_run_in_parallel(count: usize) -> bool {
    parallel_threads_for(count) > 1
}

/// Map each item through the provided operation, spreading the work across
/// scoped threads when there are enough items, otherwise running serially on
/// the calling thread. The returned list is in the same order as the items,
/// regardless of which thread processed them.
///
/// Items are claimed in small batches from a shared cursor, in the order they
/// were provided, so callers can order items to reduce cross-thread waiting
/// (for example, dependencies before their dependents). The calling thread
/// participates in the work, and the current tracing span is propagated
/// to every worker thread.
pub fn map_in_parallel<I, O, F>(items: &[I], op: F) -> Vec<O>
where
    I: Sync,
    O: Send,
    F: Fn(&I) -> O + Sync,
{
    let threads = parallel_threads_for(items.len());

    if threads <= 1 {
        return items.iter().map(op).collect();
    }

    // Small batches keep threads balanced when some items are more
    // expensive than others, while limiting contention on the cursor
    let batch_size = (items.len() / (threads * 4)).max(1);
    let cursor = AtomicUsize::new(0);
    let span = tracing::Span::current();

    let work = || {
        let _guard = span.enter();
        let mut results = Vec::with_capacity(items.len() / threads + batch_size);

        loop {
            let start = cursor.fetch_add(batch_size, Ordering::Relaxed);

            if start >= items.len() {
                break;
            }

            for (index, item) in items.iter().enumerate().skip(start).take(batch_size) {
                results.push((index, op(item)));
            }
        }

        results
    };

    let chunks = thread::scope(|scope| {
        let handles = (1..threads).map(|_| scope.spawn(work)).collect::<Vec<_>>();

        let mut chunks = vec![work()];

        for handle in handles {
            match handle.join() {
                Ok(chunk) => chunks.push(chunk),
                Err(panic) => std::panic::resume_unwind(panic),
            }
        }

        chunks
    });

    let mut slots = Vec::with_capacity(items.len());
    slots.resize_with(items.len(), || None);

    for (index, output) in chunks.into_iter().flatten() {
        slots[index] = Some(output);
    }

    slots
        .into_iter()
        .map(|slot| slot.expect("Every item is processed exactly once!"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustc_hash::FxHashSet;
    use std::sync::Mutex;

    #[test]
    fn stays_on_calling_thread_for_small_inputs() {
        let caller = thread::current().id();
        let items = (0..PARALLEL_MIN_ITEMS_PER_THREAD * 2 - 1).collect::<Vec<_>>();

        assert!(!should_run_in_parallel(items.len()));

        let threads = map_in_parallel(&items, |_| thread::current().id());

        assert!(threads.into_iter().all(|id| id == caller));
    }

    #[test]
    fn spreads_large_inputs_across_threads() {
        let items = (0..PARALLEL_MIN_ITEMS_PER_THREAD * get_concurrency() * 4).collect::<Vec<_>>();
        let seen = Mutex::new(FxHashSet::default());

        map_in_parallel(&items, |_| {
            seen.lock().unwrap().insert(thread::current().id());
            // Give other threads a chance to claim work
            thread::sleep(std::time::Duration::from_micros(50));
        });

        if get_concurrency() > 1 {
            assert!(should_run_in_parallel(items.len()));
            assert!(seen.into_inner().unwrap().len() > 1);
        }
    }

    #[test]
    fn returns_outputs_in_input_order() {
        let items = (0..1000).collect::<Vec<usize>>();

        assert_eq!(
            map_in_parallel(&items, |item| item * 2),
            items.iter().map(|item| item * 2).collect::<Vec<_>>()
        );
    }

    #[test]
    fn processes_each_item_once() {
        let items = (0..997).collect::<Vec<usize>>();
        let count = AtomicUsize::new(0);

        map_in_parallel(&items, |_| count.fetch_add(1, Ordering::Relaxed));

        assert_eq!(count.into_inner(), items.len());
    }

    #[test]
    #[should_panic(expected = "boom")]
    fn propagates_panics() {
        let items = (0..1000).collect::<Vec<usize>>();

        map_in_parallel(&items, |item| {
            if *item == 500 {
                panic!("boom");
            }
        });
    }
}
