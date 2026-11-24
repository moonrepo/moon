use std::time::Duration;
use tokio::task::JoinHandle;
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;

/// Cancel the provided token once the timeout (in seconds) has elapsed.
/// A timeout of `0` is the same as no timeout.
pub fn monitor_timeout(
    duration: Option<u64>,
    timeout_token: CancellationToken,
) -> Option<JoinHandle<()>> {
    duration.filter(|duration| *duration > 0).map(|duration| {
        tokio::spawn(async move {
            sleep(Duration::from_secs(duration)).await;
            timeout_token.cancel();
        })
    })
}
