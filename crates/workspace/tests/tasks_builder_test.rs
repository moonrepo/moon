use moon_common::Id;
use moon_config::{TaskDependencyConfig, TaskDependencyType};
use moon_graph_utils::{GraphConnections, GraphExpanderContext};
use moon_project_graph::ProjectGraph;
use moon_task::{Target, Task};
use moon_workspace::WorkspaceTasksBuilder;
use rustc_hash::FxHashMap;
use std::sync::Arc;

fn create_task(id: &str, deps: Vec<TaskDependencyConfig>) -> Task {
    Task {
        id: Id::raw(id),
        target: Target::parse(&format!("proj:{id}")).unwrap(),
        deps,
        ..Task::default()
    }
}

fn create_dep(id: &str, type_of: TaskDependencyType) -> TaskDependencyConfig {
    TaskDependencyConfig {
        target: Target::parse(&format!("proj:{id}")).unwrap(),
        type_of,
        ..TaskDependencyConfig::default()
    }
}

fn map_edges(builder: &WorkspaceTasksBuilder) -> Vec<(String, String, String)> {
    let targets = builder
        .targets_to_indexes
        .iter()
        .map(|(target, index)| (*index, target.to_string()))
        .collect::<FxHashMap<_, _>>();

    let mut edges = builder
        .graph
        .graph()
        .edge_indices()
        .map(|edge| {
            let (from, to) = builder.graph.edge_endpoints(edge).unwrap();

            (
                targets.get(&from).unwrap().to_owned(),
                targets.get(&to).unwrap().to_owned(),
                builder.graph.edge_weight(edge).unwrap().to_string(),
            )
        })
        .collect::<Vec<_>>();
    edges.sort();
    edges
}

mod tasks_builder {
    use super::*;

    #[test]
    fn maps_dep_types_to_edge_weights() {
        let mut builder = WorkspaceTasksBuilder::new();

        builder
            .build(vec![create_task(
                "task",
                vec![
                    create_dep("required", TaskDependencyType::Required),
                    create_dep("wait", TaskDependencyType::Wait),
                    create_dep("cleanup", TaskDependencyType::Cleanup),
                ],
            )])
            .unwrap();

        assert_eq!(
            map_edges(&builder),
            vec![
                // Reversed!
                ("proj:cleanup".into(), "proj:task".into(), "cleanup".into()),
                (
                    "proj:task".into(),
                    "proj:required".into(),
                    "required".into()
                ),
                ("proj:task".into(), "proj:wait".into(), "wait".into()),
            ]
        );
    }

    #[test]
    fn marks_optional_required_deps() {
        let mut builder = WorkspaceTasksBuilder::new();

        builder
            .build(vec![create_task(
                "task",
                vec![TaskDependencyConfig {
                    optional: Some(true),
                    ..create_dep("required", TaskDependencyType::Required)
                }],
            )])
            .unwrap();

        assert_eq!(
            map_edges(&builder),
            vec![(
                "proj:task".into(),
                "proj:required".into(),
                "optional".into()
            )]
        );
    }

    #[test]
    fn type_wins_over_optional() {
        let mut builder = WorkspaceTasksBuilder::new();

        builder
            .build(vec![create_task(
                "task",
                vec![
                    TaskDependencyConfig {
                        optional: Some(true),
                        ..create_dep("cleanup", TaskDependencyType::Cleanup)
                    },
                    TaskDependencyConfig {
                        optional: Some(true),
                        ..create_dep("wait", TaskDependencyType::Wait)
                    },
                ],
            )])
            .unwrap();

        assert_eq!(
            map_edges(&builder),
            vec![
                ("proj:cleanup".into(), "proj:task".into(), "cleanup".into()),
                ("proj:task".into(), "proj:wait".into(), "wait".into()),
            ]
        );
    }

    #[test]
    fn doesnt_cycle_when_a_cleanup_and_required_dep_agree() {
        let mut builder = WorkspaceTasksBuilder::new();

        // `a` cleans up after `b`, and `b` requires `a`, so both edges
        // point in the same direction (b -> a) and don't cycle
        builder
            .build(vec![
                create_task("a", vec![create_dep("b", TaskDependencyType::Cleanup)]),
                create_task("b", vec![create_dep("a", TaskDependencyType::Required)]),
            ])
            .unwrap();

        assert_eq!(
            map_edges(&builder),
            vec![
                ("proj:b".into(), "proj:a".into(), "cleanup".into()),
                ("proj:b".into(), "proj:a".into(), "required".into()),
            ]
        );
    }

    #[test]
    fn errors_when_cleanup_deps_cycle() {
        let mut builder = WorkspaceTasksBuilder::new();

        let error = builder
            .build(vec![
                create_task("a", vec![create_dep("b", TaskDependencyType::Cleanup)]),
                create_task("b", vec![create_dep("a", TaskDependencyType::Cleanup)]),
            ])
            .unwrap_err();

        assert!(
            error
                .to_string()
                .contains("adding a relationship from proj:a to proj:b would introduce a cycle"),
            "{error}"
        );
    }

    #[test]
    fn errors_when_required_deps_cycle() {
        let mut builder = WorkspaceTasksBuilder::new();

        let error = builder
            .build(vec![
                create_task("a", vec![create_dep("b", TaskDependencyType::Required)]),
                create_task("b", vec![create_dep("a", TaskDependencyType::Required)]),
            ])
            .unwrap_err();

        assert!(
            error
                .to_string()
                .contains("adding a relationship from proj:b to proj:a would introduce a cycle"),
            "{error}"
        );
    }

    // `missing` is never built, so it remains a placeholder that's indexed
    // before `lint`, which shifts down an index when it's filtered out
    #[test]
    fn reindexes_after_filtering_placeholders() {
        let mut builder = WorkspaceTasksBuilder::new();

        builder
            .build(vec![
                create_task(
                    "build",
                    vec![
                        create_dep("missing", TaskDependencyType::Required),
                        create_dep("lint", TaskDependencyType::Required),
                    ],
                ),
                create_task("lint", vec![]),
            ])
            .unwrap();

        let graph = builder.finalize(
            GraphExpanderContext::default(),
            Arc::new(ProjectGraph::default()),
        );
        let build = graph
            .get_unexpanded(&Target::parse("proj:build").unwrap())
            .unwrap();
        let lint_target = Target::parse("proj:lint").unwrap();

        let mut targets = graph
            .get_node_keys()
            .into_iter()
            .map(|target| target.to_string())
            .collect::<Vec<_>>();
        targets.sort();

        assert_eq!(targets, ["proj:build", "proj:lint"]);
        assert_eq!(graph.dependencies_of(build), vec![lint_target.clone()]);
        assert_eq!(graph.deep_dependencies_of(build), vec![lint_target]);
    }
}
