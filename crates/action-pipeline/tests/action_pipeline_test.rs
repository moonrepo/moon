use moon_action::{Action, ActionStatus};
use moon_action_graph::RunRequirements;
use moon_common::Id;
use moon_process::ProcessRegistry;
use moon_task::Target;
use moon_test_utils::WorkspaceMocker;
use moon_toolchain::ToolchainSpec;
use rustc_hash::FxHashMap;
use starbase_sandbox::{Sandbox, create_sandbox};

fn get_labels(actions: Vec<Action>) -> Vec<String> {
    actions.into_iter().map(|action| action.label).collect()
}

fn get_statuses(actions: Vec<Action>) -> FxHashMap<String, ActionStatus> {
    actions
        .into_iter()
        .map(|action| (action.label, action.status))
        .collect()
}

// Provisions a `SetupEnvironment -> InstallDependencies` chain for the
// `priority` project using the `tc-tier2-setup-env` test toolchain. The
// install writes a marker file when it runs, and the environment setup
// can be told to fail, so that tests can observe what happens downstream
fn create_setup_env_mocker(sandbox: &Sandbox, fail_setup_env: bool) -> WorkspaceMocker {
    WorkspaceMocker::new(sandbox.path())
        .with_default_projects()
        .with_test_toolchains()
        // The test plugin roots the dependencies workspace at the working dir
        .set_working_dir(sandbox.path().join("priority"))
        .update_toolchains_config(|cfg| {
            if let Some(inner) = cfg.plugins.get_mut(&Id::raw("tc-tier2-setup-env")) {
                inner
                    .config
                    .insert("testInstallMarker".into(), serde_json::json!(true));

                if fail_setup_env {
                    inner.config.insert(
                        "testSetupEnvironmentFailure".into(),
                        serde_json::json!(true),
                    );
                }
            }
        })
}

mod action_pipeline {
    use super::*;

    mod abort {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn runs_dependents_when_dependency_passes() {
            let sandbox = create_sandbox("pipeline");
            let mocker = create_setup_env_mocker(&sandbox, false);

            let spec = ToolchainSpec::new_global(Id::raw("tc-tier2-setup-env"));
            let project = mocker
                .mock_workspace_graph()
                .await
                .get_project("priority")
                .unwrap();

            let mut graph = mocker.create_action_graph().await;
            graph.install_dependencies(&spec, &project).await.unwrap();

            let (context, graph) = graph.build();
            let actions = mocker
                .mock_action_pipeline()
                .await
                .run_with_context(graph, context)
                .await
                .unwrap();

            assert_eq!(
                get_labels(actions),
                [
                    "SyncWorkspace",
                    "SetupEnvironment(tc-tier2-setup-env, priority)",
                    "InstallDependencies(tc-tier2-setup-env, priority)",
                ]
            );

            // Sanity check the marker mechanism itself, so that the
            // negative assertion in the next test actually means something
            assert!(
                sandbox
                    .path()
                    .join(".moon/cache/tcInstallDependencies")
                    .exists()
            );
        }

        // Provisioning failures abort the pipeline. The dispatcher releases
        // dependents as soon as their dependencies have *completed*, so the
        // abort must be raised before the failed job is marked as completed,
        // otherwise the dependent slips through and runs against a broken
        // environment (https://github.com/moonrepo/moon/issues/2653)
        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_run_dependents_when_dependency_aborts() {
            let sandbox = create_sandbox("pipeline");
            let mocker = create_setup_env_mocker(&sandbox, true);

            let spec = ToolchainSpec::new_global(Id::raw("tc-tier2-setup-env"));
            let project = mocker
                .mock_workspace_graph()
                .await
                .get_project("priority")
                .unwrap();

            let mut graph = mocker.create_action_graph().await;
            graph.install_dependencies(&spec, &project).await.unwrap();

            let (context, graph) = graph.build();
            let mut pipeline = mocker.mock_action_pipeline().await;
            // Some parallelism, so that a dependent *could* be dispatched
            pipeline.concurrency = 4;

            let error = pipeline.run_with_context(graph, context).await.unwrap_err();

            assert!(
                error
                    .to_string()
                    .contains("Failed to setup environment (test)"),
                "unexpected error: {error}"
            );

            assert!(
                !sandbox
                    .path()
                    .join(".moon/cache/tcInstallDependencies")
                    .exists(),
                "InstallDependencies ran even though SetupEnvironment failed"
            );
        }
    }

    mod priority {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn runs_priority_in_order() {
            let sandbox = create_sandbox("pipeline");
            let mocker = WorkspaceMocker::new(sandbox.path()).with_default_projects();

            let reqs = RunRequirements::default();
            let mut graph = mocker.create_action_graph().await;
            graph
                .run_task_by_target(&Target::parse("priority:low").unwrap(), &reqs)
                .await
                .unwrap();
            graph
                .run_task_by_target(&Target::parse("priority:normal").unwrap(), &reqs)
                .await
                .unwrap();
            graph
                .run_task_by_target(&Target::parse("priority:high").unwrap(), &reqs)
                .await
                .unwrap();
            graph
                .run_task_by_target(&Target::parse("priority:critical").unwrap(), &reqs)
                .await
                .unwrap();

            let (context, graph) = graph.build();
            let actions = mocker
                .mock_action_pipeline()
                .await
                .run_with_context(graph, context)
                .await
                .unwrap();

            assert_eq!(
                get_labels(actions),
                [
                    "SyncWorkspace",
                    "SyncProject(priority)",
                    "RunTask(priority:critical)",
                    "RunTask(priority:high)",
                    "RunTask(priority:normal)",
                    "RunTask(priority:low)"
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn critical_depends_on_low() {
            let sandbox = create_sandbox("pipeline");
            let mocker = WorkspaceMocker::new(sandbox.path()).with_default_projects();

            let reqs = RunRequirements::default();
            let mut graph = mocker.create_action_graph().await;
            graph
                .run_task_by_target(&Target::parse("priority:critical-low").unwrap(), &reqs)
                .await
                .unwrap();

            let (context, graph) = graph.build();
            let actions = mocker
                .mock_action_pipeline()
                .await
                .run_with_context(graph, context)
                .await
                .unwrap();

            assert_eq!(
                get_labels(actions),
                [
                    "SyncWorkspace",
                    "SyncProject(priority)",
                    "RunTask(priority:critical-low-base)",
                    "RunTask(priority:critical-low)"
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn high_depends_on_low() {
            let sandbox = create_sandbox("pipeline");
            let mocker = WorkspaceMocker::new(sandbox.path()).with_default_projects();

            let reqs = RunRequirements::default();
            let mut graph = mocker.create_action_graph().await;
            graph
                .run_task_by_target(&Target::parse("priority:high-low").unwrap(), &reqs)
                .await
                .unwrap();

            let (context, graph) = graph.build();
            let actions = mocker
                .mock_action_pipeline()
                .await
                .run_with_context(graph, context)
                .await
                .unwrap();

            assert_eq!(
                get_labels(actions),
                [
                    "SyncWorkspace",
                    "SyncProject(priority)",
                    "RunTask(priority:high-low-base)",
                    "RunTask(priority:high-low)"
                ]
            );
        }
    }

    mod persistent {
        use super::*;

        // Persistent tasks are dispatched topologically like any other task,
        // instead of being batched and ran at the end of the pipeline, so that
        // other tasks can run alongside them
        #[tokio::test(flavor = "multi_thread")]
        async fn runs_in_parallel_with_other_tasks() {
            let sandbox = create_sandbox("pipeline");
            let mocker = WorkspaceMocker::new(sandbox.path()).with_default_projects();

            let reqs = RunRequirements::default();
            let mut graph = mocker.create_action_graph().await;
            graph
                .run_task_by_target(&Target::parse("persistent:server").unwrap(), &reqs)
                .await
                .unwrap();
            graph
                .run_task_by_target(&Target::parse("persistent:client").unwrap(), &reqs)
                .await
                .unwrap();

            let (context, graph) = graph.build();
            let actions = mocker
                .mock_action_pipeline()
                .await
                .run_with_context(graph, context)
                .await
                .unwrap();
            let statuses = get_statuses(actions);
            assert_eq!(
                statuses.get("RunPersistentTask(persistent:server)"),
                Some(&ActionStatus::Passed)
            );
            assert_eq!(
                statuses.get("RunTask(persistent:client)"),
                Some(&ActionStatus::Passed)
            );
        }

        // Persistent tasks are marked as completed the moment they are
        // dispatched, so that they don't block other actions, but they must
        // still wait for their own dependencies to finish
        #[tokio::test(flavor = "multi_thread")]
        async fn waits_for_dependencies_to_complete() {
            let sandbox = create_sandbox("pipeline");
            let mocker = WorkspaceMocker::new(sandbox.path()).with_default_projects();

            let reqs = RunRequirements::default();
            let mut graph = mocker.create_action_graph().await;
            graph
                .run_task_by_target(
                    &Target::parse("persistent:server-with-deps").unwrap(),
                    &reqs,
                )
                .await
                .unwrap();

            let (context, graph) = graph.build();
            let actions = mocker
                .mock_action_pipeline()
                .await
                .run_with_context(graph, context)
                .await
                .unwrap();
            let statuses = get_statuses(actions);

            assert_eq!(
                statuses.get("RunTask(persistent:slow-build)"),
                Some(&ActionStatus::Passed)
            );
            assert_eq!(
                statuses.get("RunPersistentTask(persistent:server-with-deps)"),
                Some(&ActionStatus::Passed)
            );
        }

        // When dependencies run serially, the tasks ordered after a persistent
        // dependency must not deadlock waiting on it to complete
        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_block_serial_dependencies() {
            let sandbox = create_sandbox("pipeline");
            let mocker = WorkspaceMocker::new(sandbox.path()).with_default_projects();

            let reqs = RunRequirements::default();
            let mut graph = mocker.create_action_graph().await;
            graph
                .run_task_by_target(&Target::parse("persistent:serial-server").unwrap(), &reqs)
                .await
                .unwrap();

            let (context, graph) = graph.build();
            let actions = mocker
                .mock_action_pipeline()
                .await
                .run_with_context(graph, context)
                .await
                .unwrap();
            let statuses = get_statuses(actions);

            assert_eq!(
                statuses.get("RunPersistentTask(persistent:server)"),
                Some(&ActionStatus::Passed)
            );
            assert_eq!(
                statuses.get("RunTask(persistent:slow-build)"),
                Some(&ActionStatus::Passed)
            );
            assert_eq!(
                statuses.get("RunPersistentTask(persistent:serial-server)"),
                Some(&ActionStatus::Passed)
            );
        }

        // The serial dependencies that surround a persistent dependency are
        // still ordered against each other, as they do complete
        #[tokio::test(flavor = "multi_thread")]
        async fn orders_serial_dependencies_around_persistent() {
            let sandbox = create_sandbox("pipeline");
            let mocker = WorkspaceMocker::new(sandbox.path()).with_default_projects();

            let reqs = RunRequirements::default();
            let mut graph = mocker.create_action_graph().await;
            graph
                .run_task_by_target(
                    &Target::parse("persistent:serial-mixed-server").unwrap(),
                    &reqs,
                )
                .await
                .unwrap();

            let (context, graph) = graph.build();
            let actions = mocker
                .mock_action_pipeline()
                .await
                .run_with_context(graph, context)
                .await
                .unwrap();
            let statuses = get_statuses(actions);

            assert_eq!(
                statuses.get("RunTask(persistent:first-build)"),
                Some(&ActionStatus::Passed)
            );
            assert_eq!(
                statuses.get("RunTask(persistent:last-build)"),
                Some(&ActionStatus::Passed)
            );
            assert_eq!(
                statuses.get("RunPersistentTask(persistent:server)"),
                Some(&ActionStatus::Passed)
            );
            assert_eq!(
                statuses.get("RunPersistentTask(persistent:serial-mixed-server)"),
                Some(&ActionStatus::Passed)
            );
        }

        // A persistent task never completes, so it must not block the
        // persistent tasks that depend on it
        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_block_persistent_dependents() {
            let sandbox = create_sandbox("pipeline");
            let mocker = WorkspaceMocker::new(sandbox.path()).with_default_projects();

            let reqs = RunRequirements::default();
            let mut graph = mocker.create_action_graph().await;
            graph
                .run_task_by_target(
                    &Target::parse("persistent:persistent-client").unwrap(),
                    &reqs,
                )
                .await
                .unwrap();

            let (context, graph) = graph.build();
            let actions = mocker
                .mock_action_pipeline()
                .await
                .run_with_context(graph, context)
                .await
                .unwrap();
            let statuses = get_statuses(actions);

            assert_eq!(
                statuses.get("RunPersistentTask(persistent:server)"),
                Some(&ActionStatus::Passed)
            );
            assert_eq!(
                statuses.get("RunPersistentTask(persistent:persistent-client)"),
                Some(&ActionStatus::Passed)
            );
        }
    }

    mod dep_types {
        use super::*;

        async fn run_pipeline(
            sandbox: &Sandbox,
            targets: &[&str],
            bail: bool,
            concurrency: usize,
        ) -> miette::Result<Vec<Action>> {
            let mocker = WorkspaceMocker::new(sandbox.path()).with_default_projects();

            let reqs = RunRequirements::default();
            let mut graph = mocker.create_action_graph().await;

            for target in targets {
                graph
                    .run_task_by_target(&Target::parse(target).unwrap(), &reqs)
                    .await
                    .unwrap();
            }

            let (context, graph) = graph.build();
            let mut pipeline = mocker.mock_action_pipeline().await;
            pipeline.bail = bail;
            pipeline.concurrency = concurrency;

            pipeline.run_with_context(graph, context).await
        }

        async fn run_targets(
            sandbox: &Sandbox,
            targets: &[&str],
            bail: bool,
        ) -> FxHashMap<String, ActionStatus> {
            // Enough for tasks that wait on each other to run in parallel
            get_statuses(run_pipeline(sandbox, targets, bail, 4).await.unwrap())
        }

        fn has_signal(sandbox: &Sandbox, name: &str) -> bool {
            sandbox.path().join("dep-types").join(name).exists()
        }

        mod wait {
            use super::*;

            // The dependency only completes once the task has started, so this
            // deadlocks (and times out) if the task waits for it to complete
            #[tokio::test(flavor = "multi_thread")]
            async fn runs_once_the_dependency_has_started() {
                let sandbox = create_sandbox("pipeline");
                let statuses = run_targets(&sandbox, &["dep-types:client"], false).await;

                assert_eq!(
                    statuses.get("RunTask(dep-types:server)"),
                    Some(&ActionStatus::Passed)
                );
                assert_eq!(
                    statuses.get("RunTask(dep-types:client)"),
                    Some(&ActionStatus::Passed)
                );
            }

            // The dependency runs without a permit, as it may never complete
            // until the task runs, which would otherwise never get a permit
            #[tokio::test(flavor = "multi_thread")]
            async fn runs_with_a_single_permit() {
                let sandbox = create_sandbox("pipeline");
                let statuses = get_statuses(
                    run_pipeline(&sandbox, &["dep-types:client"], false, 1)
                        .await
                        .unwrap(),
                );

                assert_eq!(
                    statuses.get("RunTask(dep-types:server)"),
                    Some(&ActionStatus::Passed)
                );
                assert_eq!(
                    statuses.get("RunTask(dep-types:client)"),
                    Some(&ActionStatus::Passed)
                );
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn can_wait_on_a_persistent_task() {
                let sandbox = create_sandbox("pipeline");
                let statuses = run_targets(&sandbox, &["dep-types:persistent-client"], false).await;

                assert_eq!(
                    statuses.get("RunPersistentTask(dep-types:persistent-server)"),
                    Some(&ActionStatus::Passed)
                );
                assert_eq!(
                    statuses.get("RunTask(dep-types:persistent-client)"),
                    Some(&ActionStatus::Passed)
                );
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn skips_if_the_dependency_already_failed() {
                let sandbox = create_sandbox("pipeline");
                let statuses = run_targets(&sandbox, &["dep-types:after-crash"], false).await;

                assert_eq!(
                    statuses.get("RunTask(dep-types:crash)"),
                    Some(&ActionStatus::Failed)
                );
                assert_eq!(
                    statuses.get("RunTask(dep-types:settle)"),
                    Some(&ActionStatus::Passed)
                );
                assert_eq!(
                    statuses.get("RunTask(dep-types:after-crash)"),
                    Some(&ActionStatus::Skipped)
                );
                assert!(!has_signal(&sandbox, "after-crash-ran"));
            }
        }

        mod cleanup {
            use super::*;

            #[tokio::test(flavor = "multi_thread")]
            async fn runs_after_the_task() {
                let sandbox = create_sandbox("pipeline");
                let statuses = run_targets(&sandbox, &["dep-types:parent"], false).await;

                assert_eq!(
                    statuses.get("RunTask(dep-types:parent)"),
                    Some(&ActionStatus::Passed)
                );
                assert_eq!(
                    statuses.get("RunTask(dep-types:cleanup)"),
                    Some(&ActionStatus::Passed)
                );
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn runs_after_the_task_fails() {
                let sandbox = create_sandbox("pipeline");
                let statuses = run_targets(&sandbox, &["dep-types:failing-parent"], false).await;

                assert_eq!(
                    statuses.get("RunTask(dep-types:failing-parent)"),
                    Some(&ActionStatus::Failed)
                );
                assert_eq!(
                    statuses.get("RunTask(dep-types:failing-cleanup)"),
                    Some(&ActionStatus::Passed)
                );
                assert!(has_signal(&sandbox, "failing-cleaned"));
            }

            // The failure aborts the pipeline, which must still run the cleanup
            #[tokio::test(flavor = "multi_thread")]
            async fn runs_after_the_task_fails_when_bailing() {
                let sandbox = create_sandbox("pipeline");
                let statuses = run_targets(&sandbox, &["dep-types:failing-parent"], true).await;

                assert_eq!(
                    statuses.get("RunTask(dep-types:failing-parent)"),
                    Some(&ActionStatus::Failed)
                );
                assert_eq!(
                    statuses.get("RunTask(dep-types:failing-cleanup)"),
                    Some(&ActionStatus::Passed)
                );
                assert!(has_signal(&sandbox, "failing-cleaned"));
            }

            // The task never ran its command, so there's nothing to clean up
            #[tokio::test(flavor = "multi_thread")]
            async fn doesnt_run_if_the_task_is_skipped() {
                let sandbox = create_sandbox("pipeline");
                let statuses = run_targets(&sandbox, &["dep-types:blocked-parent"], false).await;

                assert_eq!(
                    statuses.get("RunTask(dep-types:broken)"),
                    Some(&ActionStatus::Failed)
                );
                assert_eq!(
                    statuses.get("RunTask(dep-types:blocked-parent)"),
                    Some(&ActionStatus::Skipped)
                );
                assert_eq!(
                    statuses.get("RunTask(dep-types:blocked-cleanup)"),
                    Some(&ActionStatus::Skipped)
                );
                assert!(!has_signal(&sandbox, "blocked-cleaned"));
            }

            // Unless the cleanup was explicitly requested
            #[tokio::test(flavor = "multi_thread")]
            async fn runs_when_requested_even_if_the_task_is_skipped() {
                let sandbox = create_sandbox("pipeline");
                let statuses = run_targets(
                    &sandbox,
                    &["dep-types:blocked-parent", "dep-types:blocked-cleanup"],
                    false,
                )
                .await;

                assert_eq!(
                    statuses.get("RunTask(dep-types:blocked-parent)"),
                    Some(&ActionStatus::Skipped)
                );
                assert_eq!(
                    statuses.get("RunTask(dep-types:blocked-cleanup)"),
                    Some(&ActionStatus::Passed)
                );
                assert!(has_signal(&sandbox, "blocked-cleaned"));
            }

            // Tasks that were terminated because of the abort are not the failure,
            // but they carry the error of their termination, which must not be
            // reported as the pipeline's error (this used to be discarded)
            #[tokio::test(flavor = "multi_thread")]
            async fn doesnt_report_terminated_tasks_as_the_failure() {
                let sandbox = create_sandbox("pipeline");
                let result = run_pipeline(
                    &sandbox,
                    &["dep-types:failing-parent", "dep-types:long-sibling"],
                    true,
                    4,
                )
                .await;

                let statuses = get_statuses(result.unwrap());

                assert_eq!(
                    statuses.get("RunTask(dep-types:failing-parent)"),
                    Some(&ActionStatus::Failed)
                );
                assert_eq!(
                    statuses.get("RunTask(dep-types:long-sibling)"),
                    Some(&ActionStatus::Aborted)
                );
                assert_eq!(
                    statuses.get("RunTask(dep-types:failing-cleanup)"),
                    Some(&ActionStatus::Passed)
                );
            }

            // Aborting terminates the running processes, and the process registry
            // kills the ones it tracks once its threshold has elapsed (even those
            // started afterwards), so the cleanup must not start until then
            #[tokio::test(flavor = "multi_thread")]
            async fn isnt_killed_when_the_pipeline_terminates_processes() {
                // Must be registered before anything else uses the registry
                ProcessRegistry::register(500);

                let sandbox = create_sandbox("pipeline");
                let statuses = run_targets(
                    &sandbox,
                    &["dep-types:slow-failing-parent", "dep-types:long-sibling"],
                    true,
                )
                .await;

                assert_eq!(
                    statuses.get("RunTask(dep-types:slow-failing-parent)"),
                    Some(&ActionStatus::Failed)
                );
                assert_eq!(
                    statuses.get("RunTask(dep-types:slow-cleanup)"),
                    Some(&ActionStatus::Passed)
                );
                assert!(has_signal(&sandbox, "slow-cleaned"));
            }

            // The pipeline is aborted before the task runs,
            // so there's nothing for the cleanup to clean up
            #[tokio::test(flavor = "multi_thread")]
            async fn doesnt_run_if_the_task_never_ran_when_bailing() {
                let sandbox = create_sandbox("pipeline");
                let statuses = run_targets(&sandbox, &["dep-types:blocked-parent"], true).await;

                assert_eq!(
                    statuses.get("RunTask(dep-types:broken)"),
                    Some(&ActionStatus::Failed)
                );
                assert_eq!(statuses.get("RunTask(dep-types:blocked-parent)"), None);
                assert_eq!(statuses.get("RunTask(dep-types:blocked-cleanup)"), None);
                assert!(!has_signal(&sandbox, "blocked-ran"));
                assert!(!has_signal(&sandbox, "blocked-cleaned"));
            }
        }
    }
}
