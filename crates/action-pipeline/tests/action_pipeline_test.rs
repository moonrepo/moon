use moon_action::Action;
use moon_action_graph::RunRequirements;
use moon_task::Target;
use moon_test_utils2::WorkspaceMocker;
use starbase_sandbox::create_sandbox;

fn get_labels(actions: Vec<Action>) -> Vec<String> {
    actions.into_iter().map(|action| action.label).collect()
}

mod action_pipeline {
    use super::*;

    mod priority {
        use super::*;

        #[tokio::test]
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

        #[tokio::test]
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

        #[tokio::test]
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
