mod utils;

use moon_action::{ActionStatus, AttemptType};
use moon_action_context::ActionContext;
use utils::*;

mod command_executor {
    use super::*;

    #[tokio::test]
    async fn returns_attempt_on_success() {
        let container = TaskRunnerContainer::new_os("executor").await;
        let context = ActionContext::default();

        let result = container
            .create_command_executor("success", &context)
            .await
            .execute(&context, Some("hash123"))
            .await
            .unwrap();

        // Check state
        assert!(result.error.is_none());
        assert_eq!(result.state.hash.unwrap(), "hash123");
        assert_eq!(result.state.attempt_current, 1);
        assert_eq!(result.state.attempt_total, 1);

        // Check attempt
        assert_eq!(result.attempts.len(), 1);

        let attempt = result.attempts.first().unwrap();
        let exec = attempt.execution.as_ref().unwrap();

        assert_eq!(attempt.status, ActionStatus::Passed);
        assert_eq!(attempt.type_of, AttemptType::TaskExecution);
        assert_eq!(exec.exit_code.unwrap(), 0);
        assert_eq!(exec.stdout.as_ref().unwrap().trim(), "test");
    }

    #[tokio::test]
    async fn returns_attempt_on_failure() {
        let container = TaskRunnerContainer::new_os("executor").await;
        let context = ActionContext::default();

        let result = container
            .create_command_executor("failure", &context)
            .await
            .execute(&context, Some("hash123"))
            .await
            .unwrap();

        // Check state
        assert!(result.error.is_none());
        assert_eq!(result.state.hash.unwrap(), "hash123");
        assert_eq!(result.state.attempt_current, 1);
        assert_eq!(result.state.attempt_total, 1);

        // Check attempt
        assert_eq!(result.attempts.len(), 1);

        let attempt = result.attempts.first().unwrap();
        let exec = attempt.execution.as_ref().unwrap();

        assert_eq!(attempt.status, ActionStatus::Failed);
        assert_eq!(attempt.type_of, AttemptType::TaskExecution);
        assert_eq!(exec.exit_code.unwrap(), 1);
    }

    #[tokio::test]
    async fn returns_attempts_for_each_retry() {
        let container = TaskRunnerContainer::new_os("executor").await;
        let context = ActionContext::default();

        let result = container
            .create_command_executor("retry", &context)
            .await
            .execute(&context, None)
            .await
            .unwrap();

        // Check state
        assert!(result.error.is_none());
        assert!(result.state.hash.is_none());
        assert_eq!(result.state.attempt_current, 4);
        assert_eq!(result.state.attempt_total, 4);

        // Check attempt
        assert_eq!(result.attempts.len(), 4);

        for i in 0..4 {
            let attempt = &result.attempts[i];
            let exec = attempt.execution.as_ref().unwrap();

            assert_eq!(attempt.status, ActionStatus::Failed);
            assert_eq!(attempt.type_of, AttemptType::TaskExecution);
            assert_eq!(exec.exit_code.unwrap(), 1);
        }
    }
}
