mod utils;

use moon_action::ActionStatus;
use moon_action_context::{ActionContext, TargetState};
use moon_console::TaskReportItem;
use moon_process::{ProcessSet, SignalType};
use std::time::Duration;
use tokio::time::sleep;
use utils::*;

mod command_executor {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
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

    #[tokio::test(flavor = "multi_thread")]
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

    #[tokio::test(flavor = "multi_thread")]
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

    mod expect_failure {
        use super::*;

        async fn execute(
            task_id: &str,
        ) -> (
            TaskReportItem,
            moon_task_runner::task_executor::TaskExecuteResult,
        ) {
            let container = TaskRunnerContainer::new_os("runner", task_id).await;
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

            (item, result)
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn passes_without_retrying_when_failed() {
            let (item, result) = execute("expect-failure").await;

            assert_eq!(item.attempt_current, 1);
            assert!(result.error.is_none());
            assert_eq!(result.run_state, TargetState::Passed("hash123".into()));
            assert_eq!(result.attempts.len(), 1);

            let attempt = result.attempts.first().unwrap();

            assert_eq!(attempt.status, ActionStatus::Passed);
            assert_eq!(attempt.get_exec_output().unwrap().exit_code.unwrap(), 1);
        }

        // Another attempt would only hide the fix if it happened to fail
        #[tokio::test(flavor = "multi_thread")]
        async fn fails_without_retrying_when_passed() {
            let (item, result) = execute("expect-failure-passes").await;

            assert_eq!(item.attempt_current, 1);
            assert!(result.error.is_none());
            assert_eq!(result.run_state, TargetState::Failed);
            assert_eq!(result.attempts.len(), 1);

            let attempt = result.attempts.first().unwrap();

            assert_eq!(attempt.status, ActionStatus::Failed);
            assert_eq!(attempt.get_exec_output().unwrap().exit_code.unwrap(), 0);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn fails_and_retries_when_command_not_found() {
            let (item, result) = execute("expect-failure-not-found").await;

            assert_eq!(item.attempt_current, 2);
            assert_eq!(result.run_state, TargetState::Failed);
            assert_eq!(result.attempts.len(), 2);

            for attempt in result.attempts.iter() {
                assert_eq!(attempt.status, ActionStatus::Failed);
                assert_eq!(attempt.get_exec_output().unwrap().exit_code.unwrap(), 127);
            }
        }
    }

    // Running processes are terminated when the pipeline is aborted (or receives
    // a signal), which fails the current attempt, but must not start another,
    // as its process would never be terminated. The root process set (and the
    // signal listener) is global, so this relies on nextest running each test
    // in its own process
    mod terminated {
        use super::*;
        use moon_task_runner::task_executor::TaskExecuteResult;

        async fn execute_and_terminate(
            container: &TaskRunnerContainer,
            context: &ActionContext,
            terminate: impl Future<Output = ()>,
        ) -> (TaskReportItem, TaskExecuteResult) {
            let mut item = TaskReportItem::default();
            let executor = container.create_command_executor(context).await;

            let (result, _) = tokio::join!(executor.execute(context, &mut item), async {
                // Give the first attempt a moment to start
                sleep(Duration::from_millis(500)).await;

                terminate.await;
            });

            (item, result.unwrap())
        }

        // Like the pipeline does when it's aborted
        async fn abort(context: &ActionContext) {
            context.abort_token.cancel();

            ProcessSet::root().shutdown(SignalType::Terminate).await;
        }

        fn assert_not_retried(item: TaskReportItem, result: TaskExecuteResult) {
            assert_eq!(item.attempt_current, 1);
            assert_eq!(item.attempt_total, 4);
            assert_eq!(result.run_state, TargetState::Failed);
            assert_eq!(result.attempts.len(), 1);
            assert_eq!(result.attempts[0].status, ActionStatus::Failed);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_retry_when_the_pipeline_aborts() {
            let container = TaskRunnerContainer::new_os("runner", "retry-slow").await;
            let context = ActionContext::default();

            let (item, result) = execute_and_terminate(&container, &context, abort(&context)).await;

            assert_not_retried(item, result);
        }

        // Cleanup tasks still run once the pipeline is aborted, but one that was
        // already running has its process terminated like any other
        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_retry_a_running_cleanup_when_the_pipeline_aborts() {
            let container = TaskRunnerContainer::new_os("runner", "retry-slow").await;
            let mut context = ActionContext::default();
            context
                .cleanup_targets
                .insert(container.task.target.clone());

            let (item, result) = execute_and_terminate(&container, &context, abort(&context)).await;

            assert_not_retried(item, result);
        }

        // Only the processes running at the time of the abort are terminated
        #[tokio::test(flavor = "multi_thread")]
        async fn retries_a_cleanup_that_starts_after_the_pipeline_aborts() {
            let container = TaskRunnerContainer::new_os("runner", "retry").await;
            let mut context = ActionContext::default();
            context
                .cleanup_targets
                .insert(container.task.target.clone());
            context.abort_token.cancel();

            let mut item = TaskReportItem::default();
            let result = container
                .create_command_executor(&context)
                .await
                .execute(&context, &mut item)
                .await
                .unwrap();

            assert_eq!(item.attempt_current, 4);
            assert_eq!(result.attempts.len(), 4);
        }

        // The task runs in a process group of its own, so the processes it
        // started are terminated along with it
        #[cfg(unix)]
        #[tokio::test(flavor = "multi_thread")]
        async fn terminates_the_processes_the_task_started() {
            use std::process::Command;

            let container = TaskRunnerContainer::new_os("runner", "spawns-grandchild").await;
            let context = ActionContext::default();

            execute_and_terminate(&container, &context, abort(&context)).await;

            let pid = std::fs::read_to_string(container.sandbox.path().join("unix/grandchild.pid"))
                .unwrap();
            let is_running = || {
                Command::new("kill")
                    .args(["-0", pid.trim()])
                    .status()
                    .unwrap()
                    .success()
            };

            // Give it a moment to exit and be reaped
            for _ in 0..20 {
                if !is_running() {
                    break;
                }

                sleep(Duration::from_millis(100)).await;
            }

            if is_running() {
                let _ = Command::new("kill").args(["-9", pid.trim()]).status();

                panic!(
                    "process {} started by the task is still running",
                    pid.trim()
                );
            }
        }

        #[cfg(unix)]
        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_retry_when_a_signal_is_received() {
            use moon_process::{SignalListener, kill};

            SignalListener::start();

            let container = TaskRunnerContainer::new_os("runner", "retry-slow").await;
            let context = ActionContext::default();

            let (item, result) = execute_and_terminate(&container, &context, async {
                kill(std::process::id(), SignalType::Terminate).unwrap();
            })
            .await;

            assert_not_retried(item, result);
        }
    }

    // The same applies when the task started after the pipeline was aborted
    // (it was preparing to run), as its process was never terminated
    #[tokio::test(flavor = "multi_thread")]
    async fn doesnt_retry_once_the_pipeline_is_aborted() {
        let container = TaskRunnerContainer::new_os("runner", "retry").await;
        let context = ActionContext::default();
        let mut item = TaskReportItem::default();

        context.abort_token.cancel();

        let result = container
            .create_command_executor(&context)
            .await
            .execute(&context, &mut item)
            .await
            .unwrap();

        assert_eq!(item.attempt_current, 1);
        assert_eq!(item.attempt_total, 4);
        assert_eq!(result.attempts.len(), 1);
    }

    // Unless it's a cleanup, which runs after the pipeline was aborted
    #[tokio::test(flavor = "multi_thread")]
    async fn retries_a_cleanup_once_the_pipeline_is_aborted() {
        let container = TaskRunnerContainer::new_os("runner", "retry").await;
        let mut context = ActionContext::default();
        let mut item = TaskReportItem::default();

        context
            .cleanup_targets
            .insert(container.task.target.clone());
        context.abort_token.cancel();

        let result = container
            .create_command_executor(&context)
            .await
            .execute(&context, &mut item)
            .await
            .unwrap();

        assert_eq!(item.attempt_current, 4);
        assert_eq!(item.attempt_total, 4);
        assert_eq!(result.attempts.len(), 4);
    }
}
