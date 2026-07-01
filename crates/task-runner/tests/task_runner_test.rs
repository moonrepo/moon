mod utils;

use moon_action::ActionStatus;
use moon_action_context::*;
use moon_cache::{CacheMode, Manifest};
use moon_config::{
    TaskCheck, TaskCheckConditionConfig, TaskCheckFingerprint, TaskCheckFingerprintConfig,
    TaskCheckRequirementConfig,
};
use moon_env_var::GlobalEnvBag;
use moon_hash::{ContentHasher, Digest};
use moon_task::Target;
use moon_task_runner::TaskRunner;
use moon_task_runner::output_hydrater::HydrateFrom;
use moon_time::now_millis;
use rustc_hash::FxHashSet;
use utils::*;

mod task_runner {
    use super::*;

    mod run {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn skips_if_noop() {
            let container = TaskRunnerContainer::new("runner", "base").await;
            let mut runner = container.create_runner();
            let node = container.create_action_node();
            let context = ActionContext::default();

            runner.run_with_panic(&context, &node).await.unwrap();

            assert_ne!(
                context
                    .target_states
                    .get_sync(&runner.task.target)
                    .unwrap()
                    .get(),
                &TargetState::Failed
            );
        }

        mod has_deps {
            use super::*;

            #[tokio::test(flavor = "multi_thread")]
            #[should_panic(expected = "Encountered a missing hash for task project:dep")]
            async fn errors_if_dep_hasnt_ran() {
                let container = TaskRunnerContainer::new("runner", "has-deps").await;
                let mut runner = container.create_runner();
                let node = container.create_action_node();
                let context = ActionContext::default();

                runner.run_with_panic(&context, &node).await.unwrap();
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn skips_if_dep_skipped() {
                let container = TaskRunnerContainer::new("runner", "has-deps").await;
                let mut runner = container.create_runner();
                let node = container.create_action_node();

                let context = ActionContext::default();
                context
                    .target_states
                    .insert_sync(Target::new("project", "dep").unwrap(), TargetState::Skipped)
                    .unwrap();

                runner.run_with_panic(&context, &node).await.unwrap();

                assert_eq!(
                    context
                        .target_states
                        .get_sync(&runner.task.target)
                        .unwrap()
                        .get(),
                    &TargetState::Skipped
                );
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn skips_if_dep_failed() {
                let container = TaskRunnerContainer::new("runner", "has-deps").await;
                let mut runner = container.create_runner();
                let node = container.create_action_node();

                let context = ActionContext::default();
                context
                    .target_states
                    .insert_sync(Target::new("project", "dep").unwrap(), TargetState::Failed)
                    .unwrap();

                runner.run_with_panic(&context, &node).await.unwrap();

                assert_eq!(
                    context
                        .target_states
                        .get_sync(&runner.task.target)
                        .unwrap()
                        .get(),
                    &TargetState::Skipped
                );
            }
        }

        mod with_cache {
            use super::*;

            #[tokio::test(flavor = "multi_thread")]
            async fn creates_cache_state_file() {
                let container = TaskRunnerContainer::new_os("runner", "create-file").await;
                container.sandbox.enable_git();

                let mut runner = container.create_runner();
                let node = container.create_action_node();
                let context = ActionContext::default();

                runner.run_with_panic(&context, &node).await.unwrap();

                assert!(
                    container
                        .sandbox
                        .path()
                        .join(".moon/cache/states")
                        .join(container.project_id)
                        .join("create-file/lastRun.json")
                        .exists()
                );
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn generates_a_hash() {
                let container = TaskRunnerContainer::new_os("runner", "create-file").await;
                container.sandbox.enable_git();

                let mut runner = container.create_runner();
                let node = container.create_action_node();
                let context = ActionContext::default();

                let result = runner.run_with_panic(&context, &node).await.unwrap();

                assert!(result.hash.is_some());
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn generates_a_hash_for_noop() {
                let container = TaskRunnerContainer::new_os("runner", "noop").await;
                container.sandbox.enable_git();

                let mut runner = container.create_runner();
                let node = container.create_action_node();
                let context = ActionContext::default();

                let result = runner.run_with_panic(&context, &node).await.unwrap();

                assert!(result.hash.is_some());
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn generates_same_hashes_based_on_input() {
                let container = TaskRunnerContainer::new_os("runner", "hash-inputs").await;
                container.sandbox.enable_git();

                let mut runner = container.create_runner();
                let node = container.create_action_node();
                let context = ActionContext::default();

                container
                    .sandbox
                    .create_file(format!("{}/file.txt", container.project_id), "same");

                let a = runner.run_with_panic(&context, &node).await.unwrap();
                let b = runner.run_with_panic(&context, &node).await.unwrap();

                assert_eq!(a.hash, b.hash);
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn generates_different_hashes_based_on_input() {
                let container = TaskRunnerContainer::new_os("runner", "hash-inputs").await;
                container.sandbox.enable_git();

                let mut runner = container.create_runner();
                let node = container.create_action_node();
                let context = ActionContext::default();

                container
                    .sandbox
                    .create_file(format!("{}/file.txt", container.project_id), "before");

                let a = runner.run_with_panic(&context, &node).await.unwrap();

                container
                    .sandbox
                    .create_file(format!("{}/file.txt", container.project_id), "after");

                let b = runner.run_with_panic(&context, &node).await.unwrap();

                assert_ne!(a.hash, b.hash);
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn creates_operations_for_each_step() {
                let container = TaskRunnerContainer::new_os("runner", "create-file").await;
                container.sandbox.enable_git();

                let mut runner = container.create_runner();
                let node = container.create_action_node();
                let context = ActionContext::default();

                let result = runner.run_with_panic(&context, &node).await.unwrap();

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

            #[tokio::test(flavor = "multi_thread")]
            async fn running_again_hits_the_output_cache() {
                let container = TaskRunnerContainer::new_os("runner", "create-file").await;
                container.sandbox.enable_git();

                let mut runner = container.create_runner();
                let node = container.create_action_node();
                let context = ActionContext::default();

                let before = runner.run_with_panic(&context, &node).await.unwrap();

                assert_eq!(before.operations.len(), 4);

                let result = runner.run_with_panic(&context, &node).await.unwrap();

                assert_eq!(before.hash, result.hash);
                assert_eq!(result.operations.len(), 2);
                assert!(result.operations[0].meta.is_hash_generation());
                assert!(result.operations[1].meta.is_output_hydration());
                assert_eq!(result.operations[0].status, ActionStatus::Passed);
                assert_eq!(result.operations[1].status, ActionStatus::Cached);
            }

            #[tokio::test(flavor = "multi_thread")]
            #[should_panic(expected = "defines outputs but after being ran")]
            async fn errors_if_outputs_missing() {
                let container = TaskRunnerContainer::new_os("runner", "missing-output").await;
                container.sandbox.enable_git();

                let mut runner = container.create_runner();
                let node = container.create_action_node();
                let context = ActionContext::default();

                runner.run_with_panic(&context, &node).await.unwrap();
            }

            #[tokio::test(flavor = "multi_thread")]
            #[should_panic(expected = "defines outputs but after being ran")]
            async fn errors_if_outputs_missing_via_glob() {
                let container = TaskRunnerContainer::new_os("runner", "missing-output-glob").await;
                container.sandbox.enable_git();

                let mut runner = container.create_runner();
                let node = container.create_action_node();
                let context = ActionContext::default();

                runner.run_with_panic(&context, &node).await.unwrap();
            }
        }

        mod without_cache {
            use super::*;

            #[tokio::test(flavor = "multi_thread")]
            async fn creates_cache_state_file() {
                let container = TaskRunnerContainer::new_os("runner", "without-cache").await;
                container.sandbox.enable_git();

                let mut runner = container.create_runner();
                let node = container.create_action_node();
                let context = ActionContext::default();

                runner.run_with_panic(&context, &node).await.unwrap();

                assert!(
                    container
                        .sandbox
                        .path()
                        .join(".moon/cache/states")
                        .join(container.project_id)
                        .join("without-cache/lastRun.json")
                        .exists()
                );
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn does_generate_a_hash() {
                let container = TaskRunnerContainer::new_os("runner", "without-cache").await;
                container.sandbox.enable_git();

                let mut runner = container.create_runner();
                let node = container.create_action_node();
                let context = ActionContext::default();

                let result = runner.run_with_panic(&context, &node).await.unwrap();

                assert!(result.hash.is_some());
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn running_again_reexecutes_task() {
                let container = TaskRunnerContainer::new_os("runner", "without-cache").await;
                container.sandbox.enable_git();

                let mut runner = container.create_runner();
                let node = container.create_action_node();
                let context = ActionContext::default();

                let result = runner.run_with_panic(&context, &node).await.unwrap();

                // hash + exec
                assert_eq!(result.operations.len(), 2);
                assert!(result.operations[1].meta.is_task_execution());
                assert_eq!(result.operations[1].status, ActionStatus::Passed);

                let result = runner.run_with_panic(&context, &node).await.unwrap();

                // hash + exec
                assert_eq!(result.operations.len(), 2);
                assert!(result.operations[1].meta.is_task_execution());
                assert_eq!(result.operations[1].status, ActionStatus::Passed);
            }
        }
    }

    mod is_cached {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn returns_none_by_default() {
            let container = TaskRunnerContainer::new("runner", "base").await;
            let mut runner = container.create_runner();

            assert!(runner.is_cached("hash123").await.unwrap().is_none());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn sets_the_hash_to_cache() {
            let container = TaskRunnerContainer::new("runner", "base").await;
            let mut runner = container.create_runner();

            runner.is_cached("hash123").await.unwrap();

            assert_eq!(runner.cache.data.hash, "hash123");
        }

        mod previous_output {
            use super::*;

            #[tokio::test(flavor = "multi_thread")]
            async fn returns_if_hashes_match() {
                let container = TaskRunnerContainer::new("runner", "base").await;
                let mut runner = container.create_runner();

                runner.cache.data.exit_code = 0;
                runner.cache.data.hash = "hash123".into();

                assert!(matches!(
                    runner.is_cached("hash123").await.unwrap(),
                    Some(HydrateFrom::PreviousOutput)
                ));
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn skips_if_hashes_dont_match() {
                let container = TaskRunnerContainer::new("runner", "base").await;
                let mut runner = container.create_runner();

                runner.cache.data.exit_code = 0;
                runner.cache.data.hash = "otherhash456".into();

                assert!(runner.is_cached("hash123").await.unwrap().is_none());
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn skips_if_codes_dont_match() {
                let container = TaskRunnerContainer::new("runner", "base").await;
                let mut runner = container.create_runner();

                runner.cache.data.exit_code = 2;
                runner.cache.data.hash = "hash123".into();

                assert!(runner.is_cached("hash123").await.unwrap().is_none());
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn skips_if_outputs_dont_exist() {
                let container = TaskRunnerContainer::new("runner", "outputs").await;
                let mut runner = container.create_runner();

                runner.cache.data.exit_code = 0;
                runner.cache.data.hash = "hash123".into();

                assert!(runner.is_cached("hash123").await.unwrap().is_none());
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn returns_if_outputs_do_exist() {
                let container = TaskRunnerContainer::new("runner", "base").await;
                let mut runner = container.create_runner();

                runner.cache.data.exit_code = 0;
                runner.cache.data.hash = "hash123".into();
                container.sandbox.create_file("project/file.txt", "");

                assert!(matches!(
                    runner.is_cached("hash123").await.unwrap(),
                    Some(HydrateFrom::PreviousOutput)
                ));
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn returns_none_if_non_zero_exit() {
                let container = TaskRunnerContainer::new("runner", "base").await;
                let mut runner = container.create_runner();

                runner.cache.data.exit_code = 1;

                assert!(runner.is_cached("hash123").await.unwrap().is_none());
            }
        }

        mod local_cach_legacy {
            use super::*;

            #[tokio::test(flavor = "multi_thread")]
            async fn returns_if_archive_exists() {
                let container = TaskRunnerContainer::new("runner", "base").await;
                let mut runner = container.create_runner();

                runner.state.digest = Digest::from_bytes(b"hash123").unwrap();

                container
                    .sandbox
                    .create_file(".moon/cache/outputs/hash123.tar.gz", "");

                assert!(matches!(
                    runner.is_cached("hash123").await.unwrap(),
                    Some(HydrateFrom::LocalArchive)
                ));
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn skips_if_archive_doesnt_exist() {
                let container = TaskRunnerContainer::new("runner", "base").await;
                let mut runner = container.create_runner();

                assert!(runner.is_cached("hash123").await.unwrap().is_none());
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn skips_if_cache_isnt_readable() {
                let container = TaskRunnerContainer::new("runner", "base").await;
                let mut runner = container.create_runner();

                container
                    .sandbox
                    .create_file(".moon/cache/outputs/hash123.tar.gz", "");

                container
                    .app_context
                    .cache_engine
                    .force_mode(CacheMode::Off);

                assert!(runner.is_cached("hash123").await.unwrap().is_none());

                GlobalEnvBag::instance().remove("MOON_CACHE");
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn skips_if_cache_is_writeonly() {
                let container = TaskRunnerContainer::new("runner", "base").await;
                let mut runner = container.create_runner();

                container
                    .sandbox
                    .create_file(".moon/cache/outputs/hash123.tar.gz", "");

                container
                    .app_context
                    .cache_engine
                    .force_mode(CacheMode::Write);

                assert!(runner.is_cached("hash123").await.unwrap().is_none());

                GlobalEnvBag::instance().remove("MOON_CACHE");
            }
        }

        mod local_cache {
            use super::*;

            #[tokio::test(flavor = "multi_thread")]
            async fn returns_if_action_result_in_ac() {
                let container = TaskRunnerContainer::new("runner", "outputs").await;
                let mut runner = container.create_runner();

                runner.state.local_cas_enabled = true;
                runner.state.digest = Digest::from_bytes(b"hash123").unwrap();

                container
                    .seed_manifest(&runner.state.digest, Manifest::default())
                    .await;

                assert!(matches!(
                    runner.is_cached("hash123").await.unwrap(),
                    Some(HydrateFrom::Storage(_))
                ));
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn skips_if_cache_isnt_readable() {
                let container = TaskRunnerContainer::new("runner", "outputs").await;
                let mut runner = container.create_runner();

                runner.state.local_cas_enabled = true;
                runner.state.local_cache_readable = false;
                runner.state.digest = Digest::from_bytes(b"hash123").unwrap();

                container
                    .seed_manifest(&runner.state.digest, Manifest::default())
                    .await;

                assert!(runner.is_cached("hash123").await.unwrap().is_none());
            }
        }

        mod lifetime {
            use super::*;
            use std::time::Duration;

            #[tokio::test(flavor = "multi_thread")]
            async fn returns_if_no_previous_run_time() {
                let container = TaskRunnerContainer::new("runner", "cache-lifetime").await;
                let mut runner = container.create_runner();

                runner.cache.data.exit_code = 0;
                runner.cache.data.hash = "hash123".into();
                runner.cache.data.last_run_time = 0;

                assert!(matches!(
                    runner.is_cached("hash123").await.unwrap(),
                    Some(HydrateFrom::PreviousOutput)
                ));
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn returns_if_within_the_ttl() {
                let container = TaskRunnerContainer::new("runner", "cache-lifetime").await;
                let mut runner = container.create_runner();

                runner.cache.data.exit_code = 0;
                runner.cache.data.hash = "hash123".into();
                runner.cache.data.last_run_time = now_millis();

                assert!(matches!(
                    runner.is_cached("hash123").await.unwrap(),
                    Some(HydrateFrom::PreviousOutput)
                ));
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn misses_if_passed_the_ttl() {
                let container = TaskRunnerContainer::new("runner", "cache-lifetime").await;
                let mut runner = container.create_runner();

                runner.cache.data.exit_code = 0;
                runner.cache.data.hash = "hash123".into();
                runner.cache.data.last_run_time = now_millis();

                assert!(matches!(
                    runner.is_cached("hash123").await.unwrap(),
                    Some(HydrateFrom::PreviousOutput)
                ));

                tokio::time::sleep(Duration::from_secs(1)).await;

                assert!(matches!(
                    runner.is_cached("hash123").await.unwrap(),
                    Some(HydrateFrom::PreviousOutput)
                ));

                tokio::time::sleep(Duration::from_secs(1)).await;

                // lifetime is 2 seconds
                assert!(runner.is_cached("hash123").await.unwrap().is_none());
            }
        }
    }

    mod is_dependencies_complete {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn returns_true_if_no_deps() {
            let container = TaskRunnerContainer::new("runner", "no-deps").await;
            let runner = container.create_runner();
            let context = ActionContext::default();

            assert!(runner.is_dependencies_complete(&context).unwrap());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn returns_false_if_dep_failed() {
            let container = TaskRunnerContainer::new("runner", "has-deps").await;
            let runner = container.create_runner();
            let context = ActionContext::default();

            context
                .target_states
                .insert_sync(Target::new("project", "dep").unwrap(), TargetState::Failed)
                .unwrap();

            assert!(!runner.is_dependencies_complete(&context).unwrap());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn returns_false_if_dep_skipped() {
            let container = TaskRunnerContainer::new("runner", "has-deps").await;
            let runner = container.create_runner();
            let context = ActionContext::default();

            context
                .target_states
                .insert_sync(Target::new("project", "dep").unwrap(), TargetState::Skipped)
                .unwrap();

            assert!(!runner.is_dependencies_complete(&context).unwrap());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn returns_true_if_dep_passed() {
            let container = TaskRunnerContainer::new("runner", "no-deps").await;
            let runner = container.create_runner();
            let context = ActionContext::default();

            context
                .target_states
                .insert_sync(
                    Target::new("project", "dep").unwrap(),
                    TargetState::Passed("hash123".into()),
                )
                .unwrap();

            assert!(runner.is_dependencies_complete(&context).unwrap());
        }

        #[tokio::test(flavor = "multi_thread")]
        #[should_panic(expected = "Encountered a missing hash for task project:dep")]
        async fn errors_if_dep_not_ran() {
            let container = TaskRunnerContainer::new("runner", "has-deps").await;
            let runner = container.create_runner();
            let context = ActionContext::default();

            runner.is_dependencies_complete(&context).unwrap();
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn returns_true_if_dep_is_ignored_for_target() {
            let container = TaskRunnerContainer::new("runner", "has-deps").await;
            let runner = container.create_runner();
            let mut context = ActionContext::default();

            context.ignored_dependencies.insert(
                runner.task.target.clone(),
                FxHashSet::from_iter([Target::new("project", "dep").unwrap()]),
            );

            assert!(runner.is_dependencies_complete(&context).unwrap());
        }

        #[tokio::test(flavor = "multi_thread")]
        #[should_panic(expected = "Encountered a missing hash for task project:dep")]
        async fn errors_if_dep_is_ignored_for_another_target() {
            let container = TaskRunnerContainer::new("runner", "has-deps").await;
            let runner = container.create_runner();
            let mut context = ActionContext::default();

            context.ignored_dependencies.insert(
                Target::new("project", "other").unwrap(),
                FxHashSet::from_iter([Target::new("project", "dep").unwrap()]),
            );

            runner.is_dependencies_complete(&context).unwrap();
        }
    }

    mod generate_hash {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn generates_a_hash() {
            let container = TaskRunnerContainer::new("runner", "base").await;
            container.sandbox.enable_git();

            let mut runner = container.create_runner();
            let context = ActionContext::default();
            let node = container.create_action_node();

            let hash = runner.hash(&context, &node).await.unwrap();

            // 64 bytes
            assert_eq!(hash.len(), 64);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn generates_a_different_hash_via_passthrough_args() {
            let container = TaskRunnerContainer::new("runner", "base").await;
            container.sandbox.enable_git();

            let mut runner = container.create_runner();
            let mut context = ActionContext::default();
            let node = container.create_action_node();

            let before_hash = runner.hash(&context, &node).await.unwrap();

            context
                .primary_targets
                .insert(Target::new("project", "base").unwrap());
            context.passthrough_args.push("--extra".into());

            let after_hash = runner.hash(&context, &node).await.unwrap();

            assert_ne!(before_hash, after_hash);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn creates_an_operation() {
            let container = TaskRunnerContainer::new("runner", "base").await;
            container.sandbox.enable_git();

            let mut runner = container.create_runner();
            let context = ActionContext::default();
            let node = container.create_action_node();

            runner.hash(&context, &node).await.unwrap();

            let operation = runner.operations.last().unwrap();

            assert!(operation.meta.is_hash_generation());
            assert_eq!(operation.status, ActionStatus::Passed);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn creates_a_manifest_file() {
            let container = TaskRunnerContainer::new("runner", "base").await;
            container.sandbox.enable_git();

            let mut runner = container.create_runner();
            let context = ActionContext::default();
            let node = container.create_action_node();

            let hash = runner.hash(&context, &node).await.unwrap();

            assert!(
                container
                    .sandbox
                    .path()
                    .join(".moon/cache/hashes")
                    .join(format!("{hash}.json"))
                    .exists()
            );
        }
    }

    mod execute {
        use super::*;

        fn setup_exec_state(runner: &mut TaskRunner) {
            runner.report.hash = Some("hash123".into());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn executes_and_sets_success_state() {
            let container = TaskRunnerContainer::new_os("runner", "success").await;
            container.sandbox.enable_git();

            let mut runner = container.create_runner();
            let node = container.create_action_node();
            let context = ActionContext::default();

            setup_exec_state(&mut runner);

            runner.execute(&context, &node).await.unwrap();

            assert_eq!(
                runner.state.target.as_ref().unwrap(),
                &TargetState::Passed("hash123".into())
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn executes_and_sets_success_state_without_hash() {
            let container = TaskRunnerContainer::new_os("runner", "success").await;
            container.sandbox.enable_git();

            let mut runner = container.create_runner();
            let node = container.create_action_node();
            let context = ActionContext::default();

            runner.execute(&context, &node).await.unwrap();

            assert_eq!(
                runner.state.target.as_ref().unwrap(),
                &TargetState::Passthrough
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn executes_and_sets_failed_state() {
            let container = TaskRunnerContainer::new_os("runner", "failure").await;
            container.sandbox.enable_git();

            let mut runner = container.create_runner();
            let node = container.create_action_node();
            let context = ActionContext::default();

            setup_exec_state(&mut runner);

            // Swallow panic so we can check operations
            let _ = runner.execute(&context, &node).await;

            assert_eq!(runner.state.target.as_ref().unwrap(), &TargetState::Failed);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn executes_and_creates_operation_on_success() {
            let container = TaskRunnerContainer::new_os("runner", "success").await;
            container.sandbox.enable_git();

            let mut runner = container.create_runner();
            let node = container.create_action_node();
            let context = ActionContext::default();

            setup_exec_state(&mut runner);

            runner.execute(&context, &node).await.unwrap();

            let operation = runner.operations.last().unwrap();

            assert!(operation.meta.is_task_execution());
            assert_eq!(operation.status, ActionStatus::Passed);

            let output = operation.get_exec_output().unwrap();

            assert_eq!(output.exit_code, Some(0));
            assert_eq!(output.stdout.as_ref().unwrap().trim(), "test");
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn executes_and_creates_operation_on_failure() {
            let container = TaskRunnerContainer::new_os("runner", "failure").await;
            container.sandbox.enable_git();

            let mut runner = container.create_runner();
            let node = container.create_action_node();
            let context = ActionContext::default();

            setup_exec_state(&mut runner);

            // Swallow panic so we can check operations
            let _ = runner.execute(&context, &node).await;

            let operation = runner.operations.last().unwrap();

            assert!(operation.meta.is_task_execution());
            assert_eq!(operation.status, ActionStatus::Failed);

            let output = operation.get_exec_output().unwrap();

            assert_eq!(output.exit_code, Some(1));
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn saves_stdlog_file_to_cache() {
            let container = TaskRunnerContainer::new_os("runner", "success").await;
            container.sandbox.enable_git();

            let mut runner = container.create_runner();
            let node = container.create_action_node();
            let context = ActionContext::default();

            setup_exec_state(&mut runner);

            runner.execute(&context, &node).await.unwrap();

            assert!(
                container
                    .sandbox
                    .path()
                    .join(".moon/cache/states")
                    .join(container.project_id)
                    .join("success/stdout.log")
                    .exists()
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn creates_operation_for_mutex_acquire() {
            let container = TaskRunnerContainer::new_os("runner", "with-mutex").await;
            container.sandbox.enable_git();

            let mut runner = container.create_runner();
            let node = container.create_action_node();
            let context = ActionContext::default();

            setup_exec_state(&mut runner);

            // Swallow panic so we can check operations
            let _ = runner.execute(&context, &node).await;

            let operation = runner
                .operations
                .iter()
                .find(|op| op.meta.is_mutex_acquisition())
                .unwrap();

            assert_eq!(operation.status, ActionStatus::Passed);
        }

        #[tokio::test(flavor = "multi_thread")]
        #[should_panic(expected = "failed to run")]
        async fn errors_when_task_exec_fails() {
            let container = TaskRunnerContainer::new_os("runner", "failure").await;
            container.sandbox.enable_git();

            let mut runner = container.create_runner();
            let node = container.create_action_node();
            let context = ActionContext::default();

            runner.report.hash = Some("hash123".into());
            runner.execute(&context, &node).await.unwrap();
        }
    }

    mod skip {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn creates_an_operation() {
            let container = TaskRunnerContainer::new("runner", "base").await;
            let mut runner = container.create_runner();

            runner.skip().unwrap();

            let operation = runner.operations.last().unwrap();

            assert!(operation.meta.is_task_execution());
            assert_eq!(operation.status, ActionStatus::Skipped);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn sets_skipped_state() {
            let container = TaskRunnerContainer::new("runner", "base").await;
            let mut runner = container.create_runner();

            runner.skip().unwrap();

            assert_eq!(runner.state.target.as_ref().unwrap(), &TargetState::Skipped);
        }
    }

    mod skip_noop {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn creates_an_operation() {
            let container = TaskRunnerContainer::new("runner", "base").await;
            let mut runner = container.create_runner();

            runner.skip_no_op().unwrap();

            let operation = runner.operations.last().unwrap();

            assert!(operation.meta.is_no_operation());
            assert_eq!(operation.status, ActionStatus::Passed);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn sets_passthrough_state() {
            let container = TaskRunnerContainer::new("runner", "base").await;
            let mut runner = container.create_runner();

            runner.skip_no_op().unwrap();

            assert_eq!(
                runner.state.target.as_ref().unwrap(),
                &TargetState::Passthrough
            );
        }
        #[tokio::test(flavor = "multi_thread")]
        async fn sets_completed_state() {
            let container = TaskRunnerContainer::new("runner", "base").await;
            let mut runner = container.create_runner();

            runner.report.hash = Some("hash123".into());
            runner.skip_no_op().unwrap();

            assert_eq!(
                runner.state.target.as_ref().unwrap(),
                &TargetState::Passed("hash123".into())
            );
        }
    }

    mod archive {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn creates_a_passed_operation_if_archived() {
            let container = TaskRunnerContainer::new("runner", "outputs").await;
            container.sandbox.enable_git();
            container.sandbox.create_file("project/file.txt", "");

            let mut runner = container.create_runner();
            let result = runner.archive("hash123").await.unwrap();

            assert!(result);

            let operation = runner.operations.last().unwrap();

            assert!(operation.meta.is_archive_creation());
            assert_eq!(operation.status, ActionStatus::Passed);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn can_archive_tasks_without_outputs() {
            let container = TaskRunnerContainer::new("runner", "base").await;
            container.sandbox.enable_git();

            let mut runner = container.create_runner();

            // Task has no outputs; legacy archive path still packs the
            // stdout/stderr logs and returns true.
            assert!(runner.archive("hash123").await.unwrap());
        }
    }

    mod hydrate {
        use super::*;

        mod not_cached {
            use super::*;

            #[tokio::test(flavor = "multi_thread")]
            async fn creates_a_skipped_operation_if_no_cache() {
                let container = TaskRunnerContainer::new("runner", "outputs").await;
                container.sandbox.enable_git();

                let mut runner = container.create_runner();

                let result = runner.hydrate("hash123").await.unwrap();

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

            #[tokio::test(flavor = "multi_thread")]
            async fn creates_a_cached_operation() {
                let container = TaskRunnerContainer::new("runner", "outputs").await;
                let mut runner = container.create_runner();

                setup_previous_state(&container, &mut runner);

                let result = runner.hydrate("hash123").await.unwrap();

                assert!(result);

                let operation = runner.operations.last().unwrap();

                assert!(operation.meta.is_output_hydration());
                assert_eq!(operation.status, ActionStatus::Cached);
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn sets_passed_state() {
                let container = TaskRunnerContainer::new("runner", "outputs").await;
                let mut runner = container.create_runner();

                setup_previous_state(&container, &mut runner);

                runner.hydrate("hash123").await.unwrap();

                assert_eq!(
                    runner.state.target.as_ref().unwrap(),
                    &TargetState::Passed("hash123".into())
                );
            }
        }

        mod local_cache_legacy {
            use super::*;
            use std::fs;

            fn setup_local_state(container: &TaskRunnerContainer, runner: &mut TaskRunner) {
                container.sandbox.enable_git();
                container.pack_archive();

                runner.state.digest = Digest::from_bytes(b"hash123").unwrap();
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn creates_a_cached_operation() {
                let container = TaskRunnerContainer::new("runner", "outputs").await;
                let mut runner = container.create_runner();

                setup_local_state(&container, &mut runner);

                let result = runner.hydrate("hash123").await.unwrap();

                assert!(result);

                let operation = runner.operations.last().unwrap();

                assert!(operation.meta.is_output_hydration());
                assert_eq!(operation.status, ActionStatus::Cached);
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn sets_passed_state() {
                let container = TaskRunnerContainer::new("runner", "outputs").await;
                let mut runner = container.create_runner();

                setup_local_state(&container, &mut runner);

                runner.hydrate("hash123").await.unwrap();

                assert_eq!(
                    runner.state.target.as_ref().unwrap(),
                    &TargetState::Passed("hash123".into())
                );
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn unpacks_archive_into_project() {
                let container = TaskRunnerContainer::new("runner", "outputs").await;
                let mut runner = container.create_runner();

                setup_local_state(&container, &mut runner);

                runner.hydrate("hash123").await.unwrap();

                let output_file = container.sandbox.path().join("project/file.txt");

                assert!(output_file.exists());
                assert_eq!(fs::read_to_string(output_file).unwrap(), "content");
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn loads_stdlogs_in_archive_into_operation() {
                let container = TaskRunnerContainer::new("runner", "outputs").await;
                let mut runner = container.create_runner();

                setup_local_state(&container, &mut runner);

                let result = runner.hydrate("hash123").await.unwrap();

                assert!(result);

                let operation = runner.operations.last().unwrap();
                let output = operation.get_exec_output().unwrap();

                assert_eq!(output.exit_code.unwrap(), 0);
                assert_eq!(output.stderr.as_deref().unwrap(), "stderr");
                assert_eq!(output.stdout.as_deref().unwrap(), "stdout");
            }
        }
    }

    mod hash_checks {
        use super::*;

        fn make_fingerprint_check(script: &str, hash: TaskCheckFingerprint) -> TaskCheck {
            TaskCheck::Fingerprint(TaskCheckFingerprintConfig {
                script: script.into(),
                hash,
            })
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn no_checks_does_not_hash() {
            let container = TaskRunnerContainer::new("runner", "base").await;
            let task = container.task.clone();
            let mut runner =
                TaskRunner::new(&container.app_context, &container.project, &task).unwrap();

            let mut hasher = ContentHasher::new("test");
            runner.hash_checks(&mut hasher).await.unwrap();

            let serialized = hasher.serialize().unwrap();
            assert_eq!(serialized, "[]");
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn non_fingerprint_checks_are_ignored() {
            let container = TaskRunnerContainer::new("runner", "base").await;
            let mut task = container.task.as_ref().to_owned();
            task.checks.push(TaskCheck::Requirement(
                moon_config::TaskCheckRequirementConfig {
                    script: "echo req".into(),
                },
            ));
            task.checks.push(TaskCheck::Condition(
                moon_config::TaskCheckConditionConfig {
                    script: "echo cond".into(),
                },
            ));
            let task = std::sync::Arc::new(task);
            let mut runner =
                TaskRunner::new(&container.app_context, &container.project, &task).unwrap();

            let mut hasher = ContentHasher::new("test");
            runner.hash_checks(&mut hasher).await.unwrap();

            let serialized = hasher.serialize().unwrap();
            assert_eq!(serialized, "[]");
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn fingerprint_hashes_all_output_by_default() {
            let container = TaskRunnerContainer::new("runner", "base").await;
            let mut task = container.task.as_ref().to_owned();
            task.checks.push(make_fingerprint_check(
                "echo hello",
                TaskCheckFingerprint::default(),
            ));
            let task = std::sync::Arc::new(task);
            let mut runner =
                TaskRunner::new(&container.app_context, &container.project, &task).unwrap();

            let mut hasher = ContentHasher::new("test");
            runner.hash_checks(&mut hasher).await.unwrap();

            let serialized = hasher.serialize().unwrap();
            assert_ne!(serialized, "[]");
            assert!(serialized.contains("\"stdout\":\"hello\""));
            assert!(serialized.contains("\"exit_code\":0"));
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn fingerprint_hash_stdout_only() {
            let container = TaskRunnerContainer::new("runner", "base").await;
            let mut task = container.task.as_ref().to_owned();
            task.checks.push(make_fingerprint_check(
                "echo out && echo err >&2",
                TaskCheckFingerprint::Stdout,
            ));
            let task = std::sync::Arc::new(task);
            let mut runner =
                TaskRunner::new(&container.app_context, &container.project, &task).unwrap();

            let mut hasher = ContentHasher::new("test");
            runner.hash_checks(&mut hasher).await.unwrap();

            let serialized = hasher.serialize().unwrap();
            assert!(serialized.contains("\"stdout\":\"out\""));
            assert!(!serialized.contains("\"stderr\""));
            assert!(!serialized.contains("\"exit_code\""));
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn fingerprint_hash_stderr_only() {
            let container = TaskRunnerContainer::new("runner", "base").await;
            let mut task = container.task.as_ref().to_owned();
            task.checks.push(make_fingerprint_check(
                "echo err >&2",
                TaskCheckFingerprint::Stderr,
            ));
            let task = std::sync::Arc::new(task);
            let mut runner =
                TaskRunner::new(&container.app_context, &container.project, &task).unwrap();

            let mut hasher = ContentHasher::new("test");
            runner.hash_checks(&mut hasher).await.unwrap();

            let serialized = hasher.serialize().unwrap();
            assert!(serialized.contains("\"stderr\":\"err\""));
            assert!(!serialized.contains("\"stdout\""));
            assert!(!serialized.contains("\"exit_code\""));
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn fingerprint_hash_exit_code_only() {
            let container = TaskRunnerContainer::new("runner", "base").await;
            let mut task = container.task.as_ref().to_owned();
            task.checks.push(make_fingerprint_check(
                "echo hello",
                TaskCheckFingerprint::ExitCode,
            ));
            let task = std::sync::Arc::new(task);
            let mut runner =
                TaskRunner::new(&container.app_context, &container.project, &task).unwrap();

            let mut hasher = ContentHasher::new("test");
            runner.hash_checks(&mut hasher).await.unwrap();

            let serialized = hasher.serialize().unwrap();
            assert!(serialized.contains("\"exit_code\":0"));
            assert!(!serialized.contains("\"stdout\""));
            assert!(!serialized.contains("\"stderr\""));
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn fingerprint_hash_disabled_skips_hashing() {
            let container = TaskRunnerContainer::new("runner", "base").await;
            let mut task = container.task.as_ref().to_owned();
            task.checks.push(make_fingerprint_check(
                "echo hello",
                TaskCheckFingerprint::Enabled(false),
            ));
            let task = std::sync::Arc::new(task);
            let mut runner =
                TaskRunner::new(&container.app_context, &container.project, &task).unwrap();

            let mut hasher = ContentHasher::new("test");
            runner.hash_checks(&mut hasher).await.unwrap();

            let serialized = hasher.serialize().unwrap();
            assert_eq!(serialized, "[]");
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn fingerprint_check_failure_returns_error() {
            let container = TaskRunnerContainer::new("runner", "base").await;
            let mut task = container.task.as_ref().to_owned();
            task.checks.push(make_fingerprint_check(
                "exit 1",
                TaskCheckFingerprint::default(),
            ));
            let task = std::sync::Arc::new(task);
            let mut runner =
                TaskRunner::new(&container.app_context, &container.project, &task).unwrap();

            let mut hasher = ContentHasher::new("test");
            let result = runner.hash_checks(&mut hasher).await;

            assert!(result.is_err());
            let err = result.unwrap_err().to_string();
            assert!(err.contains("failed to run fingerprint check"), "{err}");
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn fingerprint_operations_are_recorded() {
            let container = TaskRunnerContainer::new("runner", "base").await;
            let mut task = container.task.as_ref().to_owned();
            task.checks.push(make_fingerprint_check(
                "echo hello",
                TaskCheckFingerprint::default(),
            ));
            let task = std::sync::Arc::new(task);
            let mut runner =
                TaskRunner::new(&container.app_context, &container.project, &task).unwrap();

            let mut hasher = ContentHasher::new("test");
            runner.hash_checks(&mut hasher).await.unwrap();

            assert!(!runner.operations.is_empty());
            assert!(runner.operations[0].meta.is_process_execution());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn different_script_output_produces_different_hash() {
            let container = TaskRunnerContainer::new("runner", "base").await;

            // First: echo "aaa"
            let mut task_a = container.task.as_ref().to_owned();
            task_a.checks.push(make_fingerprint_check(
                "echo aaa",
                TaskCheckFingerprint::default(),
            ));
            let task_a = std::sync::Arc::new(task_a);
            let mut runner_a =
                TaskRunner::new(&container.app_context, &container.project, &task_a).unwrap();

            let mut hasher_a = ContentHasher::new("test");
            runner_a.hash_checks(&mut hasher_a).await.unwrap();
            let hash_a = hasher_a.generate_hash().unwrap();

            // Second: echo "bbb"
            let mut task_b = container.task.as_ref().to_owned();
            task_b.checks.push(make_fingerprint_check(
                "echo bbb",
                TaskCheckFingerprint::default(),
            ));
            let task_b = std::sync::Arc::new(task_b);
            let mut runner_b =
                TaskRunner::new(&container.app_context, &container.project, &task_b).unwrap();

            let mut hasher_b = ContentHasher::new("test");
            runner_b.hash_checks(&mut hasher_b).await.unwrap();
            let hash_b = hasher_b.generate_hash().unwrap();

            assert_ne!(hash_a, hash_b);
        }
    }

    mod execute_checks {
        use super::*;

        fn make_requirement(script: &str) -> TaskCheck {
            TaskCheck::Requirement(TaskCheckRequirementConfig {
                script: script.into(),
            })
        }

        fn make_condition(script: &str) -> TaskCheck {
            TaskCheck::Condition(TaskCheckConditionConfig {
                script: script.into(),
            })
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn no_checks_returns_false() {
            let container = TaskRunnerContainer::new("runner", "base").await;
            let task = container.task.clone();
            let mut runner =
                TaskRunner::new(&container.app_context, &container.project, &task).unwrap();

            let result = runner.execute_checks().await.unwrap();
            assert!(!result);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn fingerprint_checks_are_ignored() {
            let container = TaskRunnerContainer::new("runner", "base").await;
            let mut task = container.task.as_ref().to_owned();
            task.checks.push(TaskCheck::Fingerprint(TaskCheckFingerprintConfig {
                script: "echo fingerprint".into(),
                hash: TaskCheckFingerprint::default(),
            }));
            let task = std::sync::Arc::new(task);
            let mut runner =
                TaskRunner::new(&container.app_context, &container.project, &task).unwrap();

            let result = runner.execute_checks().await.unwrap();
            assert!(!result);
        }

        mod requirements {
            use super::*;

            #[tokio::test(flavor = "multi_thread")]
            async fn passes_on_zero_exit() {
                let container = TaskRunnerContainer::new("runner", "base").await;
                let mut task = container.task.as_ref().to_owned();
                task.checks.push(make_requirement("exit 0"));
                let task = std::sync::Arc::new(task);
                let mut runner =
                    TaskRunner::new(&container.app_context, &container.project, &task).unwrap();

                let result = runner.execute_checks().await.unwrap();
                assert!(!result);
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn fails_on_nonzero_exit() {
                let container = TaskRunnerContainer::new("runner", "base").await;
                let mut task = container.task.as_ref().to_owned();
                task.checks.push(make_requirement("exit 1"));
                let task = std::sync::Arc::new(task);
                let mut runner =
                    TaskRunner::new(&container.app_context, &container.project, &task).unwrap();

                let result = runner.execute_checks().await;
                assert!(result.is_err());
                let err = result.unwrap_err().to_string();
                assert!(
                    err.contains("requirement check"),
                    "expected requirement error, got: {err}"
                );
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn multiple_passing_requirements() {
                let container = TaskRunnerContainer::new("runner", "base").await;
                let mut task = container.task.as_ref().to_owned();
                task.checks.push(make_requirement("exit 0"));
                task.checks.push(make_requirement("echo ok"));
                let task = std::sync::Arc::new(task);
                let mut runner =
                    TaskRunner::new(&container.app_context, &container.project, &task).unwrap();

                let result = runner.execute_checks().await.unwrap();
                assert!(!result);
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn records_operations() {
                let container = TaskRunnerContainer::new("runner", "base").await;
                let mut task = container.task.as_ref().to_owned();
                task.checks.push(make_requirement("exit 0"));
                let task = std::sync::Arc::new(task);
                let mut runner =
                    TaskRunner::new(&container.app_context, &container.project, &task).unwrap();

                runner.execute_checks().await.unwrap();
                assert!(!runner.operations.is_empty());
                assert!(runner.operations[0].meta.is_process_execution());
            }
        }

        mod conditions {
            use super::*;

            #[tokio::test(flavor = "multi_thread")]
            async fn skips_task_when_condition_passes() {
                let container = TaskRunnerContainer::new("runner", "base").await;
                let mut task = container.task.as_ref().to_owned();
                task.checks.push(make_condition("exit 0"));
                let task = std::sync::Arc::new(task);
                let mut runner =
                    TaskRunner::new(&container.app_context, &container.project, &task).unwrap();

                let result = runner.execute_checks().await.unwrap();
                assert!(result);
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn runs_task_when_condition_fails() {
                let container = TaskRunnerContainer::new("runner", "base").await;
                let mut task = container.task.as_ref().to_owned();
                task.checks.push(make_condition("exit 1"));
                let task = std::sync::Arc::new(task);
                let mut runner =
                    TaskRunner::new(&container.app_context, &container.project, &task).unwrap();

                let result = runner.execute_checks().await.unwrap();
                assert!(!result);
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn all_conditions_must_pass_to_skip() {
                let container = TaskRunnerContainer::new("runner", "base").await;
                let mut task = container.task.as_ref().to_owned();
                task.checks.push(make_condition("exit 0"));
                task.checks.push(make_condition("exit 1"));
                let task = std::sync::Arc::new(task);
                let mut runner =
                    TaskRunner::new(&container.app_context, &container.project, &task).unwrap();

                let result = runner.execute_checks().await.unwrap();
                assert!(!result);
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn multiple_passing_conditions_skip() {
                let container = TaskRunnerContainer::new("runner", "base").await;
                let mut task = container.task.as_ref().to_owned();
                task.checks.push(make_condition("exit 0"));
                task.checks.push(make_condition("echo ok"));
                let task = std::sync::Arc::new(task);
                let mut runner =
                    TaskRunner::new(&container.app_context, &container.project, &task).unwrap();

                let result = runner.execute_checks().await.unwrap();
                assert!(result);
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn condition_failure_does_not_error() {
                let container = TaskRunnerContainer::new("runner", "base").await;
                let mut task = container.task.as_ref().to_owned();
                task.checks.push(make_condition("exit 1"));
                let task = std::sync::Arc::new(task);
                let mut runner =
                    TaskRunner::new(&container.app_context, &container.project, &task).unwrap();

                let result = runner.execute_checks().await;
                assert!(result.is_ok());
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn records_operations() {
                let container = TaskRunnerContainer::new("runner", "base").await;
                let mut task = container.task.as_ref().to_owned();
                task.checks.push(make_condition("exit 0"));
                let task = std::sync::Arc::new(task);
                let mut runner =
                    TaskRunner::new(&container.app_context, &container.project, &task).unwrap();

                runner.execute_checks().await.unwrap();
                assert!(!runner.operations.is_empty());
                assert!(runner.operations[0].meta.is_process_execution());
            }
        }

        mod mixed {
            use super::*;

            #[tokio::test(flavor = "multi_thread")]
            async fn condition_pass_with_requirement_pass_skips() {
                let container = TaskRunnerContainer::new("runner", "base").await;
                let mut task = container.task.as_ref().to_owned();
                task.checks.push(make_condition("exit 0"));
                task.checks.push(make_requirement("exit 0"));
                let task = std::sync::Arc::new(task);
                let mut runner =
                    TaskRunner::new(&container.app_context, &container.project, &task).unwrap();

                let result = runner.execute_checks().await.unwrap();
                assert!(result);
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn condition_fail_with_requirement_pass_runs() {
                let container = TaskRunnerContainer::new("runner", "base").await;
                let mut task = container.task.as_ref().to_owned();
                task.checks.push(make_condition("exit 1"));
                task.checks.push(make_requirement("exit 0"));
                let task = std::sync::Arc::new(task);
                let mut runner =
                    TaskRunner::new(&container.app_context, &container.project, &task).unwrap();

                let result = runner.execute_checks().await.unwrap();
                assert!(!result);
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn requirement_failure_errors_regardless_of_conditions() {
                let container = TaskRunnerContainer::new("runner", "base").await;
                let mut task = container.task.as_ref().to_owned();
                task.checks.push(make_condition("exit 0"));
                task.checks.push(make_requirement("exit 1"));
                let task = std::sync::Arc::new(task);
                let mut runner =
                    TaskRunner::new(&container.app_context, &container.project, &task).unwrap();

                let result = runner.execute_checks().await;
                assert!(result.is_err());
            }
        }
    }
}
