mod utils;

use moon_action::ActionStatus;
use moon_action_context::{ActionContext, TargetState};
use moon_console::TaskReportItem;
use utils::*;

mod command_executor {
    use super::*;

    #[tokio::test]
    async fn returns_attempt_on_success() {
        let container = TaskRunnerContainer::new_os("runner", "success").await;
        let context = ActionContext::default();
        let mut item = TaskReportItem {
            hash: Some("hash123".into()),
            ..TaskReportItem::default()
        };

        let result = container
            .create_command_executor(&context)
            .await
            .execute(&context, &mut item)
            .await
            .unwrap();

        // Check state
        assert_eq!(item.hash.unwrap(), "hash123");
        assert_eq!(item.attempt_current, 1);
        assert_eq!(item.attempt_total, 1);
        assert!(result.error.is_none());
        assert_eq!(result.run_state, TargetState::Passed("hash123".into()));

        // Check attempt
        assert_eq!(result.attempts.len(), 1);

        let attempt = result.attempts.first().unwrap();
        let output = attempt.get_exec_output().unwrap();

        assert_eq!(attempt.status, ActionStatus::Passed);
        assert!(attempt.meta.is_task_execution());
        assert_eq!(output.exit_code.unwrap(), 0);
        assert_eq!(output.stdout.as_ref().unwrap().trim(), "test");
    }

    #[tokio::test]
    async fn returns_attempt_on_failure() {
        let container = TaskRunnerContainer::new_os("runner", "failure").await;
        let context = ActionContext::default();
        let mut item = TaskReportItem {
            hash: Some("hash123".into()),
            ..TaskReportItem::default()
        };

        let result = container
            .create_command_executor(&context)
            .await
            .execute(&context, &mut item)
            .await
            .unwrap();

        // Check state
        assert_eq!(item.hash.unwrap(), "hash123");
        assert_eq!(item.attempt_current, 1);
        assert_eq!(item.attempt_total, 1);
        assert!(result.error.is_none());
        assert_eq!(result.run_state, TargetState::Failed);

        // Check attempt
        assert_eq!(result.attempts.len(), 1);

        let attempt = result.attempts.first().unwrap();
        let output = attempt.get_exec_output().unwrap();

        assert_eq!(attempt.status, ActionStatus::Failed);
        assert!(attempt.meta.is_task_execution());
        assert_eq!(output.exit_code.unwrap(), 1);
    }

    #[tokio::test]
    async fn returns_attempts_for_each_retry() {
        let container = TaskRunnerContainer::new_os("runner", "retry").await;
        let context = ActionContext::default();
        let mut item = TaskReportItem::default();

        let result = container
            .create_command_executor(&context)
            .await
            .execute(&context, &mut item)
            .await
            .unwrap();

        // Check state
        assert!(item.hash.is_none());
        assert_eq!(item.attempt_current, 4);
        assert_eq!(item.attempt_total, 4);
        assert!(result.error.is_none());
        assert_eq!(result.run_state, TargetState::Failed);

        // Check attempt
        assert_eq!(result.attempts.len(), 4);

        for i in 0..4 {
            let attempt = &result.attempts[i];
            let output = attempt.get_exec_output().unwrap();

            assert_eq!(attempt.status, ActionStatus::Failed);
            assert!(attempt.meta.is_task_execution());
            assert_eq!(output.exit_code.unwrap(), 1);
        }
    }
}
