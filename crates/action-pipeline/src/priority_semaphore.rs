use std::collections::{BTreeMap, VecDeque};
use std::sync::{Arc, Mutex};
use tokio::sync::oneshot;

/// A concurrency limiter that grants released permits to the highest-priority
/// waiter (lowest value, matching `ActionNode::get_priority`), and to the
/// earliest waiter within the same priority. A fair (FIFO) semaphore would
/// grant permits in dispatch order instead, which ignores `priority` once
/// actions are dispatched: a cached high-priority dependency would wait
/// behind long-running low-priority tasks that were dispatched before it.
pub struct PrioritySemaphore {
    state: Mutex<State>,
}

struct State {
    permits: usize,
    next_id: u64,
    waiters: BTreeMap<u8, VecDeque<Waiter>>,
}

struct Waiter {
    id: u64,
    sender: oneshot::Sender<()>,
}

impl PrioritySemaphore {
    pub fn new(permits: usize) -> Self {
        Self {
            state: Mutex::new(State {
                permits,
                next_id: 0,
                waiters: BTreeMap::new(),
            }),
        }
    }

    /// Acquire a permit, waiting behind higher-priority waiters (and earlier
    /// waiters of the same priority) when none is available.
    pub async fn acquire(self: Arc<Self>, priority: u8) -> Permit {
        let (receiver, mut guard) = {
            let mut state = self.state.lock().unwrap();

            if state.permits > 0 {
                state.permits -= 1;

                return Permit { semaphore: self };
            }

            let (sender, receiver) = oneshot::channel();
            let id = state.next_id;

            state.next_id += 1;
            state
                .waiters
                .entry(priority)
                .or_default()
                .push_back(Waiter { id, sender });

            let guard = WaitGuard {
                semaphore: Arc::clone(&self),
                priority,
                id,
                granted: false,
            };

            (receiver, guard)
        };

        // If the future is dropped while waiting (the job was aborted), the
        // guard removes the waiter, or returns the permit if it was already
        // granted, so that a permit is never lost
        let _ = receiver.await;
        guard.granted = true;

        Permit { semaphore: self }
    }

    fn release(&self) {
        let mut state = self.state.lock().unwrap();

        loop {
            let Some(priority) = state.waiters.keys().next().copied() else {
                state.permits += 1;

                return;
            };

            let queue = state.waiters.get_mut(&priority).unwrap();
            let waiter = queue.pop_front().unwrap();

            if queue.is_empty() {
                state.waiters.remove(&priority);
            }

            // Skip waiters whose receiver is already gone, so they don't
            // consume the permit
            if waiter.sender.send(()).is_ok() {
                return;
            }
        }
    }
}

/// A granted permit, which releases back to the highest-priority waiter
/// (or the pool) when dropped.
pub struct Permit {
    semaphore: Arc<PrioritySemaphore>,
}

impl Drop for Permit {
    fn drop(&mut self) {
        self.semaphore.release();
    }
}

/// Removes a waiter from the queue when its acquire future is dropped
/// before the permit was consumed.
struct WaitGuard {
    semaphore: Arc<PrioritySemaphore>,
    priority: u8,
    id: u64,
    granted: bool,
}

impl Drop for WaitGuard {
    fn drop(&mut self) {
        if self.granted {
            return;
        }

        let mut state = self.semaphore.state.lock().unwrap();

        if let Some(queue) = state.waiters.get_mut(&self.priority)
            && let Some(position) = queue.iter().position(|waiter| waiter.id == self.id)
        {
            queue.remove(position);

            if queue.is_empty() {
                state.waiters.remove(&self.priority);
            }

            return;
        }

        // The permit was granted, but the future was dropped before consuming
        // it, so hand it to the next waiter instead of losing it
        drop(state);
        self.semaphore.release();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::future::Future;
    use std::task::{Context, Waker};
    use tokio::sync::mpsc;

    // Regression for #2801: a permit must go to the highest-priority waiter,
    // not to the action that was dispatched (queued) first
    #[tokio::test]
    async fn permits_are_granted_in_priority_order() {
        let semaphore = Arc::new(PrioritySemaphore::new(1));
        let holder = semaphore.clone().acquire(2).await;
        let (sender, mut receiver) = mpsc::unbounded_channel();

        // Queue waiters in low -> critical order, like actions dispatched
        // while earlier-dispatched tasks are still holding the permits
        for (priority, label) in [(3, "low"), (2, "normal"), (1, "high"), (0, "critical")] {
            let semaphore = Arc::clone(&semaphore);
            let sender = sender.clone();

            tokio::spawn(async move {
                let _permit = semaphore.acquire(priority).await;

                sender.send(label).unwrap();
            });

            // Let the spawned task register as a waiter before the next one
            tokio::task::yield_now().await;
        }

        drop(holder);
        drop(sender);

        for expected in ["critical", "high", "normal", "low"] {
            assert_eq!(receiver.recv().await.unwrap(), expected);
        }
    }

    #[tokio::test]
    async fn waiters_of_same_priority_are_granted_in_dispatch_order() {
        let semaphore = Arc::new(PrioritySemaphore::new(1));
        let holder = semaphore.clone().acquire(2).await;
        let (sender, mut receiver) = mpsc::unbounded_channel();

        for label in ["first", "second", "third"] {
            let semaphore = Arc::clone(&semaphore);
            let sender = sender.clone();

            tokio::spawn(async move {
                let _permit = semaphore.acquire(2).await;

                sender.send(label).unwrap();
            });

            tokio::task::yield_now().await;
        }

        drop(holder);
        drop(sender);

        for expected in ["first", "second", "third"] {
            assert_eq!(receiver.recv().await.unwrap(), expected);
        }
    }

    #[tokio::test]
    async fn dropped_waiter_does_not_consume_a_permit() {
        let semaphore = Arc::new(PrioritySemaphore::new(1));
        let holder = semaphore.clone().acquire(2).await;
        let mut cx = Context::from_waker(Waker::noop());
        let mut waiter = Box::pin(semaphore.clone().acquire(3));

        assert!(waiter.as_mut().poll(&mut cx).is_pending());

        drop(waiter);
        drop(holder);

        let state = semaphore.state.lock().unwrap();

        assert_eq!(state.permits, 1);
        assert!(state.waiters.is_empty());
    }

    #[tokio::test]
    async fn permit_granted_to_dropped_waiter_is_returned() {
        let semaphore = Arc::new(PrioritySemaphore::new(1));
        let holder = semaphore.clone().acquire(2).await;
        let mut cx = Context::from_waker(Waker::noop());
        let mut waiter = Box::pin(semaphore.clone().acquire(3));

        assert!(waiter.as_mut().poll(&mut cx).is_pending());

        // Grant the permit to the waiter, then drop it before it resumes
        drop(holder);
        drop(waiter);

        let state = semaphore.state.lock().unwrap();

        assert_eq!(state.permits, 1);
        assert!(state.waiters.is_empty());

        drop(state);

        let mut reacquire = Box::pin(semaphore.clone().acquire(2));

        assert!(reacquire.as_mut().poll(&mut cx).is_ready());
    }
}
