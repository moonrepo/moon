use moon_task_runner::timeout_monitor::monitor_timeout;
use std::time::Duration;
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;

// https://github.com/moonrepo/moon/issues/2818 (zqwwd3)
mod monitor_timeout {
    use super::*;

    #[test]
    fn doesnt_monitor_without_a_timeout() {
        assert!(monitor_timeout(None, CancellationToken::new()).is_none());
    }

    #[test]
    fn doesnt_monitor_a_timeout_of_zero() {
        assert!(monitor_timeout(Some(0), CancellationToken::new()).is_none());
    }

    #[tokio::test(start_paused = true)]
    async fn cancels_once_elapsed() {
        let token = CancellationToken::new();
        let handle = monitor_timeout(Some(30), token.clone()).unwrap();

        sleep(Duration::from_secs(29)).await;

        assert!(!token.is_cancelled());

        sleep(Duration::from_secs(1)).await;
        handle.await.unwrap();

        assert!(token.is_cancelled());
    }

    // The timeout used to race a sleep of a day, so a longer one never fired
    #[tokio::test(start_paused = true)]
    async fn cancels_at_the_maximum_of_1_week() {
        let token = CancellationToken::new();
        let handle = monitor_timeout(Some(604800), token.clone()).unwrap();

        sleep(Duration::from_secs(604799)).await;

        assert!(!token.is_cancelled());

        sleep(Duration::from_secs(1)).await;
        handle.await.unwrap();

        assert!(token.is_cancelled());
    }
}
