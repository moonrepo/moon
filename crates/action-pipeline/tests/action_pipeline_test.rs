use moon_action::{Action, ActionStatus};
use moon_action_graph::RunRequirements;
use moon_affected::UpstreamScope;
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

    mod scopes {
        use super::*;

        // Multiple actions may run the same task (with different args), and with
        // a direct scope, its dependencies are in scope for the one that was
        // requested, but not for the one that is a dependency of another task.
        // Both must wait for them to complete regardless (the latter did not,
        // and failed, as the hash of the dependency was missing)
        #[tokio::test(flavor = "multi_thread")]
        async fn waits_for_dependencies_in_every_action_of_a_task() {
            for targets in [
                ["scopes:with-args", "scopes:shared"],
                ["scopes:shared", "scopes:with-args"],
            ] {
                let sandbox = create_sandbox("pipeline");
                let mocker = WorkspaceMocker::new(sandbox.path()).with_default_projects();

                let reqs = RunRequirements {
                    dependencies: UpstreamScope::Direct,
                    ..RunRequirements::default()
                };
                let mut graph = mocker.create_action_graph().await;

                for target in targets {
                    graph
                        .run_task_by_target(&Target::parse(target).unwrap(), &reqs)
                        .await
                        .unwrap();
                }

                let (context, graph) = graph.build();
                let actions = mocker
                    .mock_action_pipeline()
                    .await
                    .run_with_context(graph, context)
                    .await
                    .unwrap_or_else(|error| panic!("{targets:?}: {error}"));

                let statuses = |label: &str| {
                    actions
                        .iter()
                        .filter(|action| action.label == label)
                        .map(|action| action.status)
                        .collect::<Vec<_>>()
                };

                assert_eq!(
                    statuses("RunTask(scopes:prep)"),
                    [ActionStatus::Passed],
                    "{targets:?}"
                );
                assert_eq!(
                    statuses("RunTask(scopes:shared)"),
                    [ActionStatus::Passed, ActionStatus::Passed],
                    "{targets:?}"
                );
                assert_eq!(
                    statuses("RunTask(scopes:with-args)"),
                    [ActionStatus::Passed],
                    "{targets:?}"
                );
            }
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
            run_pipeline_with_reqs(
                sandbox,
                targets,
                bail,
                concurrency,
                RunRequirements::default(),
            )
            .await
        }

        async fn run_pipeline_with_reqs(
            sandbox: &Sandbox,
            targets: &[&str],
            bail: bool,
            concurrency: usize,
            reqs: RunRequirements,
        ) -> miette::Result<Vec<Action>> {
            let mocker = WorkspaceMocker::new(sandbox.path()).with_default_projects();

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

        fn remove_signal(sandbox: &Sandbox, name: &str) {
            let _ = std::fs::remove_file(sandbox.path().join("dep-types").join(name));
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

            // The dependency never starts (its own dependency failed), so the
            // task must be skipped as well, instead of running without it
            #[tokio::test(flavor = "multi_thread")]
            async fn skips_if_the_dependency_is_skipped() {
                let sandbox = create_sandbox("pipeline");
                let statuses =
                    run_targets(&sandbox, &["dep-types:blocked-server-client"], false).await;

                assert_eq!(
                    statuses.get("RunTask(dep-types:blocked-server)"),
                    Some(&ActionStatus::Skipped)
                );
                assert_eq!(
                    statuses.get("RunTask(dep-types:blocked-server-client)"),
                    Some(&ActionStatus::Skipped)
                );
                assert!(!has_signal(&sandbox, "blocked-server-ran"));
                assert!(!has_signal(&sandbox, "blocked-server-client-ran"));

                // Nothing was started, so there's nothing to stop either
                assert_eq!(
                    statuses.get("RunTask(dep-types:blocked-server-stop)"),
                    Some(&ActionStatus::Skipped)
                );
                assert!(!has_signal(&sandbox, "blocked-server-stopped"));
            }

            // A persistent dependency is marked as completed once dispatched,
            // but it's also skipped when its own dependency failed
            #[tokio::test(flavor = "multi_thread")]
            async fn skips_if_the_persistent_dependency_is_skipped() {
                let sandbox = create_sandbox("pipeline");
                let statuses =
                    run_targets(&sandbox, &["dep-types:blocked-persistent-client"], false).await;

                assert_eq!(
                    statuses.get("RunPersistentTask(dep-types:blocked-persistent-server)"),
                    Some(&ActionStatus::Skipped)
                );
                assert_eq!(
                    statuses.get("RunTask(dep-types:blocked-persistent-client)"),
                    Some(&ActionStatus::Skipped)
                );
                assert!(!has_signal(&sandbox, "blocked-persistent-server-ran"));
                assert!(!has_signal(&sandbox, "blocked-persistent-client-ran"));

                // Nothing was started, so there's nothing to stop either
                assert_eq!(
                    statuses.get("RunTask(dep-types:blocked-server-stop)"),
                    Some(&ActionStatus::Skipped)
                );
                assert!(!has_signal(&sandbox, "blocked-server-stopped"));
            }

            // An interactive task runs in isolation, but must not wait on its
            // dependency to complete, as it only does once the cleanup has stopped
            // it, and the cleanup is dispatched after the task (this deadlocked
            // until the dependency timed out)
            #[tokio::test(flavor = "multi_thread")]
            async fn doesnt_deadlock_an_interactive_task() {
                let sandbox = create_sandbox("pipeline");
                let statuses =
                    run_targets(&sandbox, &["dep-types:interactive-client"], false).await;

                assert_eq!(
                    statuses.get("RunInteractiveTask(dep-types:interactive-client)"),
                    Some(&ActionStatus::Passed)
                );
                assert_eq!(
                    statuses.get("RunTask(dep-types:stop-server)"),
                    Some(&ActionStatus::Passed)
                );
                assert_eq!(
                    statuses.get("RunTask(dep-types:stoppable-server)"),
                    Some(&ActionStatus::Passed)
                );
            }

            // The same applies when the entire pipeline is interactive
            #[tokio::test(flavor = "multi_thread")]
            async fn doesnt_deadlock_an_interactive_pipeline() {
                let sandbox = create_sandbox("pipeline");
                let statuses = get_statuses(
                    run_pipeline_with_reqs(
                        &sandbox,
                        &["dep-types:stoppable-client"],
                        false,
                        4,
                        RunRequirements {
                            interactive: true,
                            ..RunRequirements::default()
                        },
                    )
                    .await
                    .unwrap(),
                );

                assert_eq!(
                    statuses.get("RunInteractiveTask(dep-types:stoppable-client)"),
                    Some(&ActionStatus::Passed)
                );
                assert_eq!(
                    statuses.get("RunInteractiveTask(dep-types:stop-server)"),
                    Some(&ActionStatus::Passed)
                );
                assert_eq!(
                    statuses.get("RunInteractiveTask(dep-types:stoppable-server)"),
                    Some(&ActionStatus::Passed)
                );
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

            // A task without a command (which only orchestrates its
            // dependencies) still counts as having ran
            #[tokio::test(flavor = "multi_thread")]
            async fn runs_after_a_noop_task() {
                let sandbox = create_sandbox("pipeline");
                let statuses = run_targets(&sandbox, &["dep-types:noop-parent"], false).await;

                assert_eq!(
                    statuses.get("RunTask(dep-types:noop-parent)"),
                    Some(&ActionStatus::Passed)
                );
                assert_eq!(
                    statuses.get("RunTask(dep-types:noop-cleanup)"),
                    Some(&ActionStatus::Passed)
                );
                assert!(has_signal(&sandbox, "noop-cleaned"));
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
                    statuses.get("RunTask(dep-types:failing-cleanup)"),
                    Some(&ActionStatus::Passed)
                );

                // The sibling is only reported when its process was terminated
                // before the cleanup completed. On Windows it isn't terminated,
                // and runs until its own timeout instead, after the pipeline has
                // stopped receiving results
                let sibling = statuses.get("RunTask(dep-types:long-sibling)");

                if cfg!(windows) {
                    assert!(matches!(sibling, None | Some(ActionStatus::Aborted)));
                } else {
                    assert_eq!(sibling, Some(&ActionStatus::Aborted));
                }
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

            // The task is skipped, but it has already started the dependency that
            // it waits on, which is stopped by the cleanup, so there's something
            // to clean up after all (the dependency never completed otherwise)
            #[tokio::test(flavor = "multi_thread")]
            async fn runs_if_the_skipped_task_started_a_wait_dependency() {
                let sandbox = create_sandbox("pipeline");
                let statuses = run_targets(&sandbox, &["dep-types:blocked-client"], false).await;

                assert_eq!(
                    statuses.get("RunTask(dep-types:blocked-client)"),
                    Some(&ActionStatus::Skipped)
                );
                assert_eq!(
                    statuses.get("RunTask(dep-types:stop-server)"),
                    Some(&ActionStatus::Passed)
                );
                assert_eq!(
                    statuses.get("RunTask(dep-types:stoppable-server)"),
                    Some(&ActionStatus::Passed)
                );
            }

            // The same applies when the task is hydrated from the cache, while the
            // dependencies that run alongside it, and after it, are never hydrated
            #[tokio::test(flavor = "multi_thread")]
            async fn runs_if_the_cached_task_started_a_wait_dependency() {
                let sandbox = create_sandbox("pipeline");
                sandbox.enable_git();

                let statuses = run_targets(&sandbox, &["dep-types:cached-client"], false).await;

                assert_eq!(
                    statuses.get("RunTask(dep-types:cached-client)"),
                    Some(&ActionStatus::Passed)
                );

                remove_signal(&sandbox, "stoppable-started");
                remove_signal(&sandbox, "stoppable-stop");

                let statuses = run_targets(&sandbox, &["dep-types:cached-client"], false).await;

                assert_eq!(
                    statuses.get("RunTask(dep-types:cached-client)"),
                    Some(&ActionStatus::Cached)
                );
                assert_eq!(
                    statuses.get("RunTask(dep-types:stop-server)"),
                    Some(&ActionStatus::Passed)
                );
                assert_eq!(
                    statuses.get("RunTask(dep-types:stoppable-server)"),
                    Some(&ActionStatus::Passed)
                );
                assert!(has_signal(&sandbox, "stoppable-started"));
                assert!(has_signal(&sandbox, "stoppable-stop"));
            }

            // The cleanup has not changed since it last ran, but it must run its
            // command every time that its task does, instead of being hydrated
            #[tokio::test(flavor = "multi_thread")]
            async fn runs_every_time_instead_of_being_hydrated() {
                let sandbox = create_sandbox("pipeline");
                sandbox.enable_git();

                for _ in 0..2 {
                    remove_signal(&sandbox, "cacheable-cleaned");

                    let statuses =
                        run_targets(&sandbox, &["dep-types:uncached-parent"], false).await;

                    assert_eq!(
                        statuses.get("RunTask(dep-types:uncached-parent)"),
                        Some(&ActionStatus::Passed)
                    );
                    assert_eq!(
                        statuses.get("RunTask(dep-types:cacheable-cleanup)"),
                        Some(&ActionStatus::Passed)
                    );
                    assert!(has_signal(&sandbox, "cacheable-cleaned"));
                }
            }

            // The first cleanup has nothing to clean up (its task never ran), but
            // the second one does, and requires it, so both must run
            #[tokio::test(flavor = "multi_thread")]
            async fn runs_a_cleanup_that_another_cleanup_requires_when_bailing() {
                // Must be registered before anything else uses the registry
                ProcessRegistry::register(500);

                let sandbox = create_sandbox("pipeline");
                let statuses = run_targets(
                    &sandbox,
                    &["dep-types:gated-parent", "dep-types:crashing-parent"],
                    true,
                )
                .await;

                assert_eq!(
                    statuses.get("RunTask(dep-types:crashing-parent)"),
                    Some(&ActionStatus::Failed)
                );
                assert_eq!(
                    statuses.get("RunTask(dep-types:first-cleanup)"),
                    Some(&ActionStatus::Passed)
                );
                assert_eq!(
                    statuses.get("RunTask(dep-types:second-cleanup)"),
                    Some(&ActionStatus::Passed)
                );
                assert!(!has_signal(&sandbox, "gated-ran"));
            }
        }

        mod abort {
            use super::*;

            // The task was dispatched before the pipeline was aborted, but was still
            // waiting on a mutex, so its command has yet to run, and must not once it
            // acquires the mutex, as nothing would terminate its process
            #[tokio::test(flavor = "multi_thread")]
            async fn doesnt_run_a_task_that_was_waiting_on_a_mutex() {
                // Must be registered before anything else uses the registry
                ProcessRegistry::register(500);

                let sandbox = create_sandbox("pipeline");
                let statuses = run_targets(
                    &sandbox,
                    &[
                        "dep-types:mutex-holder",
                        "dep-types:mutex-waiter",
                        "dep-types:mutex-crash",
                    ],
                    true,
                )
                .await;

                // In case its process was started, and is still running
                tokio::time::sleep(std::time::Duration::from_millis(750)).await;

                assert_eq!(
                    statuses.get("RunTask(dep-types:mutex-crash)"),
                    Some(&ActionStatus::Failed)
                );
                assert!(has_signal(&sandbox, "delay-done"));
                assert!(!has_signal(&sandbox, "holder-done"));
                assert!(!has_signal(&sandbox, "waiter-ran"));
            }
        }
    }
}
