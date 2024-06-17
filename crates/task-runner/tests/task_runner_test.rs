mod utils;

use moon_action::ActionStatus;
use moon_action_context::*;
use moon_cache::CacheMode;
use moon_task::Target;
use moon_task_runner::output_hydrater::HydrateFrom;
use moon_task_runner::TaskRunner;
use std::env;
use utils::*;

mod task_runner {
    use super::*;

    mod run {
        use super::*;

        #[tokio::test]
        async fn skips_if_noop() {
            let container = TaskRunnerContainer::new("runner").await;
            let mut runner = container.create_runner("base");
            let node = container.create_action_node("base");
            let context = ActionContext::default();

            runner.run(&context, &node).await.unwrap();

            assert_eq!(
                context
                    .target_states
                    .get(&runner.task.target)
                    .unwrap()
                    .get(),
                &TargetState::Passthrough
            );
        }

        mod has_deps {
            use super::*;

            #[tokio::test]
            #[should_panic(expected = "Encountered a missing hash for task project:dep")]
            async fn errors_if_dep_hasnt_ran() {
                let container = TaskRunnerContainer::new("runner").await;
                let mut runner = container.create_runner("has-deps");
                let node = container.create_action_node("has-deps");
                let context = ActionContext::default();

                runner.run(&context, &node).await.unwrap();
            }

            #[tokio::test]
            async fn skips_if_dep_skipped() {
                let container = TaskRunnerContainer::new("runner").await;
                let mut runner = container.create_runner("has-deps");
                let node = container.create_action_node("has-deps");

                let context = ActionContext::default();
                context
                    .target_states
                    .insert(Target::new("project", "dep").unwrap(), TargetState::Skipped)
                    .unwrap();

                runner.run(&context, &node).await.unwrap();

                assert_eq!(
                    context
                        .target_states
                        .get(&runner.task.target)
                        .unwrap()
                        .get(),
                    &TargetState::Skipped
                );
            }

            #[tokio::test]
            async fn skips_if_dep_failed() {
                let container = TaskRunnerContainer::new("runner").await;
                let mut runner = container.create_runner("has-deps");
                let node = container.create_action_node("has-deps");

                let context = ActionContext::default();
                context
                    .target_states
                    .insert(Target::new("project", "dep").unwrap(), TargetState::Failed)
                    .unwrap();

                runner.run(&context, &node).await.unwrap();

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

        mod with_cache {
            use super::*;

            #[tokio::test]
            async fn creates_cache_state_file() {
                let container = TaskRunnerContainer::new_os("runner").await;
                container.sandbox.enable_git();

                let mut runner = container.create_runner("create-file");
                let node = container.create_action_node("create-file");
                let context = ActionContext::default();

                runner.run(&context, &node).await.unwrap();

                assert!(container
                    .sandbox
                    .path()
                    .join(".moon/cache/states")
                    .join(container.project_id)
                    .join("create-file/lastRun.json")
                    .exists());
            }

            #[tokio::test]
            async fn generates_a_hash() {
                let container = TaskRunnerContainer::new_os("runner").await;
                container.sandbox.enable_git();

                let mut runner = container.create_runner("create-file");
                let node = container.create_action_node("create-file");
                let context = ActionContext::default();

                let result = runner.run(&context, &node).await.unwrap();

                assert!(result.hash.is_some());
            }

            #[tokio::test]
            async fn generates_a_hash_for_noop() {
                let container = TaskRunnerContainer::new_os("runner").await;
                container.sandbox.enable_git();

                let mut runner = container.create_runner("noop");
                let node = container.create_action_node("noop");
                let context = ActionContext::default();

                let result = runner.run(&context, &node).await.unwrap();

                assert!(result.hash.is_some());
            }

            #[tokio::test]
            async fn generates_same_hashes_based_on_input() {
                let container = TaskRunnerContainer::new_os("runner").await;
                container.sandbox.enable_git();

                let mut runner = container.create_runner("hash-inputs");
                let node = container.create_action_node("hash-inputs");
                let context = ActionContext::default();

                container
                    .sandbox
                    .create_file(format!("{}/file.txt", container.project_id), "same");

                let a = runner.run(&context, &node).await.unwrap();
                let b = runner.run(&context, &node).await.unwrap();

                assert_eq!(a.hash, b.hash);
            }

            #[tokio::test]
            async fn generates_different_hashes_based_on_input() {
                let container = TaskRunnerContainer::new_os("runner").await;
                container.sandbox.enable_git();

                let mut runner = container.create_runner("hash-inputs");
                let node = container.create_action_node("hash-inputs");
                let context = ActionContext::default();

                container
                    .sandbox
                    .create_file(format!("{}/file.txt", container.project_id), "before");

                let a = runner.run(&context, &node).await.unwrap();

                container
                    .sandbox
                    .create_file(format!("{}/file.txt", container.project_id), "after");

                let b = runner.run(&context, &node).await.unwrap();

                assert_ne!(a.hash, b.hash);
            }

            #[tokio::test]
            async fn creates_operations_for_each_step() {
                let container = TaskRunnerContainer::new_os("runner").await;
                container.sandbox.enable_git();

                let mut runner = container.create_runner("create-file");
                let node = container.create_action_node("create-file");
                let context = ActionContext::default();

                let result = runner.run(&context, &node).await.unwrap();

                assert_eq!(result.operations.len(), 4);
                assert!(result.operations[0].meta.is_hash_generation());
                assert!(result.operations[1].meta.is_output_hydration());
                assert!(result.operations[2].meta.is_task_execution());
                assert!(result.operations[3].meta.is_archive_creation());
                assert_eq!(result.operations[0].status, ActionStatus::Passed);
                assert_eq!(result.operations[1].status, ActionStatus::Skipped);
                assert_eq!(result.operations[2].status, ActionStatus::Passed);
                assert_eq!(result.operations[3].status, ActionStatus::Passed);
            }

            #[tokio::test]
            async fn running_again_hits_the_output_cache() {
                let container = TaskRunnerContainer::new_os("runner").await;
                container.sandbox.enable_git();

                let mut runner = container.create_runner("create-file");
                let node = container.create_action_node("create-file");
                let context = ActionContext::default();

                let before = runner.run(&context, &node).await.unwrap();

                assert_eq!(before.operations.len(), 4);

                let result = runner.run(&context, &node).await.unwrap();

                assert_eq!(before.hash, result.hash);
                assert_eq!(result.operations.len(), 2);
                assert!(result.operations[0].meta.is_hash_generation());
                assert!(result.operations[1].meta.is_output_hydration());
                assert_eq!(result.operations[0].status, ActionStatus::Passed);
                assert_eq!(result.operations[1].status, ActionStatus::Cached);
            }

            #[tokio::test]
            #[should_panic(expected = "defines outputs, but none exist")]
            async fn errors_if_outputs_missing() {
                let container = TaskRunnerContainer::new_os("runner").await;
                container.sandbox.enable_git();

                let mut runner = container.create_runner("missing-output");
                let node = container.create_action_node("missing-output");
                let context = ActionContext::default();

                runner.run(&context, &node).await.unwrap();
            }

            #[tokio::test]
            #[should_panic(expected = "defines outputs, but none exist")]
            async fn errors_if_outputs_missing_via_glob() {
                let container = TaskRunnerContainer::new_os("runner").await;
                container.sandbox.enable_git();

                let mut runner = container.create_runner("missing-output-glob");
                let node = container.create_action_node("missing-output-glob");
                let context = ActionContext::default();

                runner.run(&context, &node).await.unwrap();
            }
        }

        mod without_cache {
            use super::*;

            #[tokio::test]
            async fn creates_cache_state_file() {
                let container = TaskRunnerContainer::new_os("runner").await;
                container.sandbox.enable_git();

                let mut runner = container.create_runner("without-cache");
                let node = container.create_action_node("without-cache");
                let context = ActionContext::default();

                runner.run(&context, &node).await.unwrap();

                assert!(container
                    .sandbox
                    .path()
                    .join(".moon/cache/states")
                    .join(container.project_id)
                    .join("without-cache/lastRun.json")
                    .exists());
            }

            #[tokio::test]
            async fn doesnt_generate_a_hash() {
                let container = TaskRunnerContainer::new_os("runner").await;
                container.sandbox.enable_git();

                let mut runner = container.create_runner("without-cache");
                let node = container.create_action_node("without-cache");
                let context = ActionContext::default();

                let result = runner.run(&context, &node).await.unwrap();

                assert!(result.hash.is_none());
            }

            #[tokio::test]
            async fn doesnt_create_non_task_operations() {
                let container = TaskRunnerContainer::new_os("runner").await;
                container.sandbox.enable_git();

                let mut runner = container.create_runner("without-cache");
                let node = container.create_action_node("without-cache");
                let context = ActionContext::default();

                let result = runner.run(&context, &node).await.unwrap();

                assert!(result
                    .operations
                    .iter()
                    .all(|op| op.meta.is_task_execution()));
            }

            #[tokio::test]
            async fn running_again_reexecutes_task() {
                let container = TaskRunnerContainer::new_os("runner").await;
                container.sandbox.enable_git();

                let mut runner = container.create_runner("without-cache");
                let node = container.create_action_node("without-cache");
                let context = ActionContext::default();

                let result = runner.run(&context, &node).await.unwrap();

                assert_eq!(result.operations.len(), 1);
                assert!(result.operations[0].meta.is_task_execution());
                assert_eq!(result.operations[0].status, ActionStatus::Passed);

                let result = runner.run(&context, &node).await.unwrap();

                assert_eq!(result.operations.len(), 1);
                assert!(result.operations[0].meta.is_task_execution());
                assert_eq!(result.operations[0].status, ActionStatus::Passed);
            }
        }
    }

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

            #[tokio::test]
            async fn returns_none_if_non_zero_exit() {
                let container = TaskRunnerContainer::new("runner").await;
                let mut runner = container.create_runner("base");

                runner.cache.data.exit_code = 1;

                assert_eq!(runner.is_cached("hash123").await.unwrap(), None);
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

                container
                    .app_context
                    .cache_engine
                    .force_mode(CacheMode::Off);

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

                container
                    .app_context
                    .cache_engine
                    .force_mode(CacheMode::Write);

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
                .insert(Target::new("project", "dep").unwrap(), TargetState::Failed)
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
                .insert(Target::new("project", "dep").unwrap(), TargetState::Skipped)
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
                    Target::new("project", "dep").unwrap(),
                    TargetState::Passed("hash123".into()),
                )
                .unwrap();

            assert!(runner.is_dependencies_complete(&context).unwrap());
        }

        #[tokio::test]
        #[should_panic(expected = "Encountered a missing hash for task project:dep")]
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

            assert!(operation.meta.is_hash_generation());
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

            assert!(operation.meta.is_task_execution());
            assert_eq!(operation.status, ActionStatus::Passed);

            let output = operation.get_output().unwrap();

            assert_eq!(output.exit_code, Some(0));
            assert_eq!(output.stdout.as_ref().unwrap().trim(), "test");
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

            assert!(operation.meta.is_task_execution());
            assert_eq!(operation.status, ActionStatus::Failed);

            let output = operation.get_output().unwrap();

            assert_eq!(output.exit_code, Some(1));
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
                .find(|op| op.meta.is_mutex_acquisition())
                .unwrap();

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

            assert!(operation.meta.is_task_execution());
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

            runner.skip_no_op(&context, None).unwrap();

            let operation = runner.operations.last().unwrap();

            assert!(operation.meta.is_no_operation());
            assert_eq!(operation.status, ActionStatus::Passed);
        }

        #[tokio::test]
        async fn sets_passthrough_state() {
            let container = TaskRunnerContainer::new("runner").await;
            let mut runner = container.create_runner("base");
            let context = ActionContext::default();

            runner.skip_no_op(&context, None).unwrap();

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
        async fn sets_completed_state() {
            let container = TaskRunnerContainer::new("runner").await;
            let mut runner = container.create_runner("base");
            let context = ActionContext::default();

            runner.skip_no_op(&context, Some("hash123")).unwrap();

            assert_eq!(
                context
                    .target_states
                    .get(&runner.task.target)
                    .unwrap()
                    .get(),
                &TargetState::Passed("hash123".into())
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

            assert!(operation.meta.is_archive_creation());
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

            assert!(operation.meta.is_archive_creation());
            assert_eq!(operation.status, ActionStatus::Skipped);
        }

        #[tokio::test]
        async fn can_archive_tasks_without_outputs() {
            let mut container = TaskRunnerContainer::new("runner").await;
            container.sandbox.enable_git();

            if let Some(config) = Arc::get_mut(&mut container.app_context.workspace_config) {
                config
                    .runner
                    .archivable_targets
                    .push(Target::new("project", "base").unwrap());
            }

            let mut runner = container.create_runner("base");

            assert!(runner.archive("hash123").await.unwrap());
        }
    }

    mod hydrate {
        use super::*;

        mod not_cached {
            use super::*;

            #[tokio::test]
            async fn creates_a_skipped_operation_if_no_cache() {
                let container = TaskRunnerContainer::new("runner").await;
                container.sandbox.enable_git();

                let mut runner = container.create_runner("outputs");

                let context = ActionContext::default();
                let result = runner.hydrate(&context, "hash123").await.unwrap();

                assert!(!result);

                let operation = runner.operations.last().unwrap();

                assert!(operation.meta.is_output_hydration());
                assert_eq!(operation.status, ActionStatus::Skipped);
            }
        }

        mod previous_output {
            use super::*;

            fn setup_previous_state(container: &TaskRunnerContainer, runner: &mut TaskRunner) {
                container.sandbox.enable_git();
                container.sandbox.create_file("project/file.txt", "");

                runner.cache.data.exit_code = 0;
                runner.cache.data.hash = "hash123".into();
            }

            #[tokio::test]
            async fn creates_a_cached_operation() {
                let container = TaskRunnerContainer::new("runner").await;
                let mut runner = container.create_runner("outputs");

                setup_previous_state(&container, &mut runner);

                let context = ActionContext::default();
                let result = runner.hydrate(&context, "hash123").await.unwrap();

                assert!(result);

                let operation = runner.operations.last().unwrap();

                assert!(operation.meta.is_output_hydration());
                assert_eq!(operation.status, ActionStatus::Cached);
            }

            #[tokio::test]
            async fn sets_passed_state() {
                let container = TaskRunnerContainer::new("runner").await;
                let mut runner = container.create_runner("outputs");

                setup_previous_state(&container, &mut runner);

                let context = ActionContext::default();
                runner.hydrate(&context, "hash123").await.unwrap();

                assert_eq!(
                    context
                        .target_states
                        .get(&runner.task.target)
                        .unwrap()
                        .get(),
                    &TargetState::Passed("hash123".into())
                );
            }
        }

        mod local_cache {
            use std::fs;

            use super::*;

            fn setup_local_state(container: &TaskRunnerContainer, _runner: &mut TaskRunner) {
                container.sandbox.enable_git();
                container.pack_archive("outputs");
            }

            #[tokio::test]
            async fn creates_a_cached_operation() {
                let container = TaskRunnerContainer::new("runner").await;
                let mut runner = container.create_runner("outputs");

                setup_local_state(&container, &mut runner);

                let context = ActionContext::default();
                let result = runner.hydrate(&context, "hash123").await.unwrap();

                assert!(result);

                let operation = runner.operations.last().unwrap();

                assert!(operation.meta.is_output_hydration());
                assert_eq!(operation.status, ActionStatus::Cached);
            }

            #[tokio::test]
            async fn sets_passed_state() {
                let container = TaskRunnerContainer::new("runner").await;
                let mut runner = container.create_runner("outputs");

                setup_local_state(&container, &mut runner);

                let context = ActionContext::default();
                runner.hydrate(&context, "hash123").await.unwrap();

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
            async fn unpacks_archive_into_project() {
                let container = TaskRunnerContainer::new("runner").await;
                let mut runner = container.create_runner("outputs");

                setup_local_state(&container, &mut runner);

                let context = ActionContext::default();
                runner.hydrate(&context, "hash123").await.unwrap();

                let output_file = container.sandbox.path().join("project/file.txt");

                assert!(output_file.exists());
                assert_eq!(fs::read_to_string(output_file).unwrap(), "content");
            }

            #[tokio::test]
            async fn loads_stdlogs_in_archive_into_operation() {
                let container = TaskRunnerContainer::new("runner").await;
                let mut runner = container.create_runner("outputs");

                setup_local_state(&container, &mut runner);

                let context = ActionContext::default();
                let result = runner.hydrate(&context, "hash123").await.unwrap();

                assert!(result);

                let operation = runner.operations.last().unwrap();
                let output = operation.get_output().unwrap();

                assert_eq!(output.exit_code.unwrap(), 0);
                assert_eq!(output.stderr.as_deref().unwrap(), "stderr");
                assert_eq!(output.stdout.as_deref().unwrap(), "stdout");
            }
        }
    }
}
