use moon_action::Action;
use moon_action_graph::RunRequirements;
use moon_common::Id;
use moon_task::Target;
use moon_test_utils::WorkspaceMocker;
use moon_toolchain::ToolchainSpec;
use starbase_sandbox::{Sandbox, create_sandbox};

fn get_labels(actions: Vec<Action>) -> Vec<String> {
    actions.into_iter().map(|action| action.label).collect()
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
}
