mod utils;

use moon_action::{ActionStatus, OperationType};
use moon_action_context::*;
use moon_task::Target;
use moon_task_runner::output_hydrater::HydrateFrom;
use std::env;
use utils::*;

mod task_runner {
    use super::*;

    mod is_cached {
        use super::*;

        #[tokio::test]
        async fn returns_none_by_default() {
            let container = TaskRunnerContainer::new("runner").await;
            let mut runner = container.create_runner("base");

            assert_eq!(runner.is_cached("hash123").await.unwrap(), None);
        }

        #[tokio::test]
        async fn sets_the_hash_to_cache() {
            let container = TaskRunnerContainer::new("runner").await;
            let mut runner = container.create_runner("base");

            runner.is_cached("hash123").await.unwrap();

            assert_eq!(runner.cache.data.hash, "hash123");
        }

        mod previous_output {
            use super::*;

            #[tokio::test]
            async fn returns_if_hashes_match() {
                let container = TaskRunnerContainer::new("runner").await;
                let mut runner = container.create_runner("base");

                runner.cache.data.exit_code = 0;
                runner.cache.data.hash = "hash123".into();

                assert_eq!(
                    runner.is_cached("hash123").await.unwrap(),
                    Some(HydrateFrom::PreviousOutput)
                );
            }

            #[tokio::test]
            async fn skips_if_hashes_dont_match() {
                let container = TaskRunnerContainer::new("runner").await;
                let mut runner = container.create_runner("base");

                runner.cache.data.exit_code = 0;
                runner.cache.data.hash = "otherhash456".into();

                assert_eq!(runner.is_cached("hash123").await.unwrap(), None);
            }

            #[tokio::test]
            async fn skips_if_codes_dont_match() {
                let container = TaskRunnerContainer::new("runner").await;
                let mut runner = container.create_runner("base");

                runner.cache.data.exit_code = 2;
                runner.cache.data.hash = "hash123".into();

                assert_eq!(runner.is_cached("hash123").await.unwrap(), None);
            }

            #[tokio::test]
            async fn skips_if_outputs_dont_exist() {
                let container = TaskRunnerContainer::new("runner").await;
                let mut runner = container.create_runner("outputs");

                runner.cache.data.exit_code = 0;
                runner.cache.data.hash = "hash123".into();

                assert_eq!(runner.is_cached("hash123").await.unwrap(), None);
            }

            #[tokio::test]
            async fn returns_if_outputs_do_exist() {
                let container = TaskRunnerContainer::new("runner").await;
                let mut runner = container.create_runner("base");

                runner.cache.data.exit_code = 0;
                runner.cache.data.hash = "hash123".into();
                container.sandbox.create_file("project/file.txt", "");

                assert_eq!(
                    runner.is_cached("hash123").await.unwrap(),
                    Some(HydrateFrom::PreviousOutput)
                );
            }
        }

        mod local_cache {
            use super::*;

            #[tokio::test]
            async fn returns_if_archive_exists() {
                let container = TaskRunnerContainer::new("runner").await;
                let mut runner = container.create_runner("base");

                container
                    .sandbox
                    .create_file(".moon/cache/outputs/hash123.tar.gz", "");

                assert_eq!(
                    runner.is_cached("hash123").await.unwrap(),
                    Some(HydrateFrom::LocalCache)
                );
            }

            #[tokio::test]
            async fn skips_if_archive_doesnt_exist() {
                let container = TaskRunnerContainer::new("runner").await;
                let mut runner = container.create_runner("base");

                assert_eq!(runner.is_cached("hash123").await.unwrap(), None);
            }

            #[tokio::test]
            async fn skips_if_cache_isnt_readable() {
                let container = TaskRunnerContainer::new("runner").await;
                let mut runner = container.create_runner("base");

                container
                    .sandbox
                    .create_file(".moon/cache/outputs/hash123.tar.gz", "");

                env::set_var("MOON_CACHE", "off");

                assert_eq!(runner.is_cached("hash123").await.unwrap(), None);

                env::remove_var("MOON_CACHE");
            }

            #[tokio::test]
            async fn skips_if_cache_is_writeonly() {
                let container = TaskRunnerContainer::new("runner").await;
                let mut runner = container.create_runner("base");

                container
                    .sandbox
                    .create_file(".moon/cache/outputs/hash123.tar.gz", "");

                env::set_var("MOON_CACHE", "write");

                assert_eq!(runner.is_cached("hash123").await.unwrap(), None);

                env::remove_var("MOON_CACHE");
            }
        }
    }

    mod is_dependencies_complete {
        use super::*;

        #[tokio::test]
        async fn returns_true_if_no_deps() {
            let container = TaskRunnerContainer::new("runner").await;
            let runner = container.create_runner("no-deps");
            let context = ActionContext::default();

            assert!(runner.is_dependencies_complete(&context).unwrap());
        }

        #[tokio::test]
        async fn returns_false_if_dep_failed() {
            let container = TaskRunnerContainer::new("runner").await;
            let runner = container.create_runner("has-deps");
            let context = ActionContext::default();

            context
                .target_states
                .insert(Target::new("project", "base").unwrap(), TargetState::Failed)
                .unwrap();

            assert!(!runner.is_dependencies_complete(&context).unwrap());
        }

        #[tokio::test]
        async fn returns_false_if_dep_skipped() {
            let container = TaskRunnerContainer::new("runner").await;
            let runner = container.create_runner("has-deps");
            let context = ActionContext::default();

            context
                .target_states
                .insert(
                    Target::new("project", "base").unwrap(),
                    TargetState::Skipped,
                )
                .unwrap();

            assert!(!runner.is_dependencies_complete(&context).unwrap());
        }

        #[tokio::test]
        async fn returns_true_if_dep_passed() {
            let container = TaskRunnerContainer::new("runner").await;
            let runner = container.create_runner("no-deps");
            let context = ActionContext::default();

            context
                .target_states
                .insert(
                    Target::new("project", "base").unwrap(),
                    TargetState::Passed("hash123".into()),
                )
                .unwrap();

            assert!(runner.is_dependencies_complete(&context).unwrap());
        }

        #[tokio::test]
        #[should_panic(expected = "Encountered a missing hash for target project:base")]
        async fn errors_if_dep_not_ran() {
            let container = TaskRunnerContainer::new("runner").await;
            let runner = container.create_runner("has-deps");
            let context = ActionContext::default();

            runner.is_dependencies_complete(&context).unwrap();
        }
    }

    mod generate_hash {
        use super::*;

        #[tokio::test]
        async fn generates_a_hash() {
            let container = TaskRunnerContainer::new("runner").await;
            container.sandbox.enable_git();

            let mut runner = container.create_runner("base");
            let context = ActionContext::default();
            let node = container.create_action_node("base");

            let hash = runner.generate_hash(&context, &node).await.unwrap();

            // 64 bytes
            assert_eq!(hash.len(), 64);
        }

        #[tokio::test]
        async fn generates_a_different_hash_via_passthrough_args() {
            let container = TaskRunnerContainer::new("runner").await;
            container.sandbox.enable_git();

            let mut runner = container.create_runner("base");
            let mut context = ActionContext::default();
            let node = container.create_action_node("base");

            let before_hash = runner.generate_hash(&context, &node).await.unwrap();

            context
                .primary_targets
                .insert(Target::new("project", "base").unwrap());
            context.passthrough_args.push("--extra".into());

            let after_hash = runner.generate_hash(&context, &node).await.unwrap();

            assert_ne!(before_hash, after_hash);
        }

        #[tokio::test]
        async fn creates_an_operation() {
            let container = TaskRunnerContainer::new("runner").await;
            container.sandbox.enable_git();

            let mut runner = container.create_runner("base");
            let context = ActionContext::default();
            let node = container.create_action_node("base");

            runner.generate_hash(&context, &node).await.unwrap();

            let operation = runner.operations.last().unwrap();

            assert_eq!(operation.type_of, OperationType::HashGeneration);
            assert_eq!(operation.status, ActionStatus::Passed);
        }

        #[tokio::test]
        async fn creates_a_manifest_file() {
            let container = TaskRunnerContainer::new("runner").await;
            container.sandbox.enable_git();

            let mut runner = container.create_runner("base");
            let context = ActionContext::default();
            let node = container.create_action_node("base");

            let hash = runner.generate_hash(&context, &node).await.unwrap();

            assert!(container
                .sandbox
                .path()
                .join(".moon/cache/hashes")
                .join(format!("{hash}.json"))
                .exists());
        }
    }

    mod execute {
        use super::*;

        #[tokio::test]
        async fn executes_and_sets_success_state() {
            let container = TaskRunnerContainer::new_os("runner").await;
            container.sandbox.enable_git();

            let mut runner = container.create_runner("success");
            let node = container.create_action_node("success");
            let context = ActionContext::default();

            runner
                .execute(&context, &node, Some("hash123"))
                .await
                .unwrap();

            assert_eq!(
                context
                    .target_states
                    .get(&runner.task.target)
                    .unwrap()
                    .get(),
                &TargetState::Passed("hash123".into())
            );
        }

        #[tokio::test]
        async fn executes_and_sets_success_state_without_hash() {
            let container = TaskRunnerContainer::new_os("runner").await;
            container.sandbox.enable_git();

            let mut runner = container.create_runner("success");
            let node = container.create_action_node("success");
            let context = ActionContext::default();

            runner.execute(&context, &node, None).await.unwrap();

            assert_eq!(
                context
                    .target_states
                    .get(&runner.task.target)
                    .unwrap()
                    .get(),
                &TargetState::Passthrough
            );
        }

        #[tokio::test]
        async fn executes_and_sets_failed_state() {
            let container = TaskRunnerContainer::new_os("runner").await;
            container.sandbox.enable_git();

            let mut runner = container.create_runner("failure");
            let node = container.create_action_node("failure");
            let context = ActionContext::default();

            // Swallow panic so we can check operations
            let _ = runner.execute(&context, &node, Some("hash123")).await;

            assert_eq!(
                context
                    .target_states
                    .get(&runner.task.target)
                    .unwrap()
                    .get(),
                &TargetState::Failed
            );
        }

        #[tokio::test]
        async fn executes_and_creates_operation_on_success() {
            let container = TaskRunnerContainer::new_os("runner").await;
            container.sandbox.enable_git();

            let mut runner = container.create_runner("success");
            let node = container.create_action_node("success");
            let context = ActionContext::default();

            runner
                .execute(&context, &node, Some("hash123"))
                .await
                .unwrap();

            let operation = runner.operations.last().unwrap();

            assert_eq!(operation.type_of, OperationType::TaskExecution);
            assert_eq!(operation.status, ActionStatus::Passed);

            let exec = operation.execution.as_ref().unwrap();

            assert_eq!(exec.exit_code, Some(0));
            assert_eq!(exec.stdout.as_ref().unwrap().trim(), "test");
        }

        #[tokio::test]
        async fn executes_and_creates_operation_on_failure() {
            let container = TaskRunnerContainer::new_os("runner").await;
            container.sandbox.enable_git();

            let mut runner = container.create_runner("failure");
            let node = container.create_action_node("failure");
            let context = ActionContext::default();

            // Swallow panic so we can check operations
            let _ = runner.execute(&context, &node, Some("hash123")).await;

            let operation = runner.operations.last().unwrap();

            assert_eq!(operation.type_of, OperationType::TaskExecution);
            assert_eq!(operation.status, ActionStatus::Failed);

            let exec = operation.execution.as_ref().unwrap();

            assert_eq!(exec.exit_code, Some(1));
        }

        #[tokio::test]
        async fn saves_stdlog_file_to_cache() {
            let container = TaskRunnerContainer::new_os("runner").await;
            container.sandbox.enable_git();

            let mut runner = container.create_runner("success");
            let node = container.create_action_node("success");
            let context = ActionContext::default();

            runner
                .execute(&context, &node, Some("hash123"))
                .await
                .unwrap();

            assert!(container
                .sandbox
                .path()
                .join(".moon/cache/states")
                .join(container.project_id)
                .join("success/stdout.log")
                .exists());
        }

        #[tokio::test]
        async fn creates_operation_for_mutex_acquire() {
            let container = TaskRunnerContainer::new_os("runner").await;
            container.sandbox.enable_git();

            let mut runner = container.create_runner("with-mutex");
            let node = container.create_action_node("with-mutex");
            let context = ActionContext::default();

            // Swallow panic so we can check operations
            let _ = runner.execute(&context, &node, Some("hash123")).await;

            let operation = runner
                .operations
                .iter()
                .find(|a| matches!(a.type_of, OperationType::MutexAcquisition))
                .unwrap();

            assert_eq!(operation.type_of, OperationType::MutexAcquisition);
            assert_eq!(operation.status, ActionStatus::Passed);
        }

        #[tokio::test]
        #[should_panic(expected = "failed to run")]
        async fn errors_when_task_exec_fails() {
            let container = TaskRunnerContainer::new_os("runner").await;
            container.sandbox.enable_git();

            let mut runner = container.create_runner("failure");
            let node = container.create_action_node("failure");
            let context = ActionContext::default();

            runner
                .execute(&context, &node, Some("hash123"))
                .await
                .unwrap();
        }
    }

    mod skip {
        use super::*;

        #[tokio::test]
        async fn creates_an_operation() {
            let container = TaskRunnerContainer::new("runner").await;
            let mut runner = container.create_runner("base");
            let context = ActionContext::default();

            runner.skip(&context).unwrap();

            let operation = runner.operations.last().unwrap();

            assert_eq!(operation.type_of, OperationType::TaskExecution);
            assert_eq!(operation.status, ActionStatus::Skipped);
        }

        #[tokio::test]
        async fn sets_skipped_state() {
            let container = TaskRunnerContainer::new("runner").await;
            let mut runner = container.create_runner("base");
            let context = ActionContext::default();

            runner.skip(&context).unwrap();

            assert_eq!(
                context
                    .target_states
                    .get(&runner.task.target)
                    .unwrap()
                    .get(),
                &TargetState::Skipped
            );
        }
    }

    mod skip_noop {
        use super::*;

        #[tokio::test]
        async fn creates_an_operation() {
            let container = TaskRunnerContainer::new("runner").await;
            let mut runner = container.create_runner("base");
            let context = ActionContext::default();

            runner.skip_noop(&context).unwrap();

            let operation = runner.operations.last().unwrap();

            assert_eq!(operation.type_of, OperationType::NoOperation);
            assert_eq!(operation.status, ActionStatus::Passed);
        }

        #[tokio::test]
        async fn sets_passthrough_state() {
            let container = TaskRunnerContainer::new("runner").await;
            let mut runner = container.create_runner("base");
            let context = ActionContext::default();

            runner.skip_noop(&context).unwrap();

            assert_eq!(
                context
                    .target_states
                    .get(&runner.task.target)
                    .unwrap()
                    .get(),
                &TargetState::Passthrough
            );
        }
    }

    mod archive {
        use super::*;
        use std::sync::Arc;

        #[tokio::test]
        async fn creates_a_passed_operation_if_archived() {
            let container = TaskRunnerContainer::new("runner").await;
            container.sandbox.enable_git();
            container.sandbox.create_file("project/file.txt", "");

            let mut runner = container.create_runner("outputs");
            let result = runner.archive("hash123").await.unwrap();

            assert!(result);

            let operation = runner.operations.last().unwrap();

            assert_eq!(operation.type_of, OperationType::ArchiveCreation);
            assert_eq!(operation.status, ActionStatus::Passed);
        }

        #[tokio::test]
        async fn creates_a_skipped_operation_if_not_archiveable() {
            let container = TaskRunnerContainer::new("runner").await;
            container.sandbox.enable_git();

            let mut runner = container.create_runner("base");
            let result = runner.archive("hash123").await.unwrap();

            assert!(!result);

            let operation = runner.operations.last().unwrap();

            assert_eq!(operation.type_of, OperationType::ArchiveCreation);
            assert_eq!(operation.status, ActionStatus::Skipped);
        }

        #[tokio::test]
        async fn can_archive_tasks_without_outputs() {
            let mut container = TaskRunnerContainer::new("runner").await;
            container.sandbox.enable_git();

            if let Some(config) = Arc::get_mut(&mut container.workspace.config) {
                config
                    .runner
                    .archivable_targets
                    .push(Target::new("project", "base").unwrap());
            }

            let mut runner = container.create_runner("base");

            assert!(runner.archive("hash123").await.unwrap());
        }
    }
}
