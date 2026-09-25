use rustc_hash::FxHashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Instant;
use tokio_util::sync::CancellationToken;

/// State for handling an aborted pipeline, shared between the pipeline, the
/// dispatcher, and jobs, so that cleanup jobs can still run once everything
/// else has been stopped.
#[derive(Debug, Default)]
pub struct AbortState {
    /// Jobs (by node index) that were aborted while running, because the
    /// pipeline was aborted, and not because they failed themselves.
    casualties: Mutex<FxHashSet<usize>>,

    /// Whether cleanup jobs should be ran (drained) after the abort.
    drain: AtomicBool,

    /// Cancelled once all cleanup jobs have been drained.
    drained: CancellationToken,

    /// When to stop waiting on the (non-cleanup) jobs that were running at the
    /// time of the abort, as they should have been terminated by then. Their
    /// cleanup jobs are not ran if they're still running.
    give_up_at: OnceLock<Instant>,

    /// Cancelled once the pipeline has handled the abort, by terminating all
    /// running processes, and determining whether to drain cleanup jobs.
    handled: CancellationToken,

    /// When jobs may start new processes, after running processes have been
    /// terminated. The process registry force kills *all* of the processes it
    /// tracks once its shutdown threshold has elapsed, including those that
    /// were started after the termination, so cleanup jobs must wait for it.
    resume_at: OnceLock<Instant>,

    /// Whether the pipeline has terminated running processes itself, which
    /// is broadcast like an external signal, but must not be treated as one.
    terminated: AtomicBool,
}

impl AbortState {
    /// Mark the abort as handled, whether to drain cleanup jobs, and when to
    /// resume starting processes, and to give up on running jobs.
    /// Only the first call has an effect.
    pub fn handle(&self, drain: bool, resume_at: Option<Instant>, give_up_at: Option<Instant>) {
        if self.handled.is_cancelled() {
            return;
        }

        self.drain.store(drain, Ordering::Release);

        if let Some(at) = resume_at {
            let _ = self.resume_at.set(at);
        }

        if let Some(at) = give_up_at {
            let _ = self.give_up_at.set(at);
        }

        self.handled.cancel();
    }

    /// Whether the abort has been handled, and cleanup jobs should be drained.
    pub fn should_drain(&self) -> bool {
        self.handled.is_cancelled() && self.drain.load(Ordering::Acquire)
    }

    pub fn get_give_up_at(&self) -> Option<Instant> {
        self.give_up_at.get().copied()
    }

    pub async fn wait_until_handled(&self) {
        self.handled.cancelled().await;
    }

    pub fn mark_drained(&self) {
        self.drained.cancel();
    }

    pub async fn wait_until_drained(&self) {
        self.drained.cancelled().await;
    }

    /// Mark a job as aborted because of the pipeline, and not itself.
    pub fn mark_casualty(&self, index: usize) {
        if let Ok(mut casualties) = self.casualties.lock() {
            casualties.insert(index);
        }
    }

    pub fn is_casualty(&self, index: usize) -> bool {
        self.casualties
            .lock()
            .is_ok_and(|casualties| casualties.contains(&index))
    }

    /// Mark that the pipeline is about to terminate running processes itself.
    pub fn mark_terminated(&self) {
        self.terminated.store(true, Ordering::Release);
    }

    /// Consume the termination marked by [`Self::mark_terminated`], returning
    /// whether there was one, so that it's only accounted for once.
    pub fn take_terminated(&self) -> bool {
        self.terminated.swap(false, Ordering::AcqRel)
    }

    /// Wait until the abort has been handled, and new processes can be started
    /// without being terminated, or until the pipeline is cancelled (a signal).
    pub async fn wait_until_resumable(&self, cancel_token: &CancellationToken) {
        tokio::select! {
            _ = self.handled.cancelled() => {}
            _ = cancel_token.cancelled() => return,
        };

        if let Some(at) = self.resume_at.get() {
            tokio::select! {
                _ = tokio::time::sleep_until((*at).into()) => {}
                _ = cancel_token.cancelled() => {}
            };
        }
    }
}

/// Marks cleanup jobs as drained when dropped, so that the pipeline never waits
/// on a drain that will never happen (the dispatcher returned early or panicked).
pub struct DrainedGuard(pub Arc<AbortState>);

impl Drop for DrainedGuard {
    fn drop(&mut self) {
        self.0.mark_drained();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn only_handles_once() {
        let state = AbortState::default();

        assert!(!state.should_drain());

        state.handle(true, None, None);
        state.handle(false, None, None);

        assert!(state.should_drain());
    }

    #[test]
    fn takes_termination_once() {
        let state = AbortState::default();

        assert!(!state.take_terminated());

        state.mark_terminated();

        assert!(state.take_terminated());
        assert!(!state.take_terminated());
    }

    #[test]
    fn tracks_casualties() {
        let state = AbortState::default();

        state.mark_casualty(1);

        assert!(state.is_casualty(1));
        assert!(!state.is_casualty(2));
    }

    #[tokio::test]
    async fn waits_until_resumable() {
        let state = AbortState::default();
        let cancel_token = CancellationToken::new();
        let at = Instant::now() + Duration::from_millis(100);

        state.handle(true, Some(at), None);
        state.wait_until_resumable(&cancel_token).await;

        assert!(Instant::now() >= at);
    }

    #[tokio::test]
    async fn stops_waiting_when_cancelled() {
        let state = AbortState::default();
        let cancel_token = CancellationToken::new();

        state.handle(true, Some(Instant::now() + Duration::from_secs(60)), None);
        cancel_token.cancel();

        let start = Instant::now();
        state.wait_until_resumable(&cancel_token).await;

        assert!(start.elapsed() < Duration::from_secs(1));
    }
}
