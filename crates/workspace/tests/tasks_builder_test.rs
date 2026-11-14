use moon_common::Id;
use moon_config::{TaskDependencyConfig, TaskDependencyType};
use moon_graph_utils::{GraphConnections, GraphExpanderContext};
use moon_project_graph::ProjectGraph;
use moon_task::{Target, Task};
use moon_workspace::WorkspaceTasksBuilder;
use rustc_hash::FxHashMap;
use starbase_utils::json::serde_json;
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

    fn build_cycle_error(tasks: Vec<Task>) -> String {
        WorkspaceTasksBuilder::new()
            .build(tasks)
            .unwrap_err()
            .to_string()
    }

    #[test]
    fn errors_with_the_edge_that_closes_a_two_node_cycle() {
        // The first edge in insertion order that closes the cycle is
        // reported, which depends on the order the tasks are built in
        assert_eq!(
            build_cycle_error(vec![
                create_task("a", vec![create_dep("b", TaskDependencyType::Required)]),
                create_task("b", vec![create_dep("a", TaskDependencyType::Required)]),
            ]),
            "Unable to create task graph, adding a relationship from proj:b to proj:a would introduce a cycle."
        );

        assert_eq!(
            build_cycle_error(vec![
                create_task("b", vec![create_dep("a", TaskDependencyType::Required)]),
                create_task("a", vec![create_dep("b", TaskDependencyType::Required)]),
            ]),
            "Unable to create task graph, adding a relationship from proj:a to proj:b would introduce a cycle."
        );
    }

    #[test]
    fn errors_with_the_last_inserted_edge_of_a_chain_cycle() {
        assert_eq!(
            build_cycle_error(vec![
                create_task("a", vec![create_dep("b", TaskDependencyType::Required)]),
                create_task("b", vec![create_dep("c", TaskDependencyType::Required)]),
                create_task("c", vec![create_dep("a", TaskDependencyType::Required)]),
            ]),
            "Unable to create task graph, adding a relationship from proj:c to proj:a would introduce a cycle."
        );

        // Not the "back" edge in task order, but the one inserted last,
        // and edges inserted after it (another cycle) are not reported
        assert_eq!(
            build_cycle_error(vec![
                create_task("c", vec![create_dep("a", TaskDependencyType::Required)]),
                create_task("a", vec![create_dep("b", TaskDependencyType::Required)]),
                create_task(
                    "b",
                    vec![
                        create_dep("c", TaskDependencyType::Required),
                        create_dep("d", TaskDependencyType::Required),
                    ]
                ),
                create_task("d", vec![create_dep("b", TaskDependencyType::Required)]),
            ]),
            "Unable to create task graph, adding a relationship from proj:b to proj:c would introduce a cycle."
        );
    }

    // `TaskDepsBuilder` drops (required) or rejects (other types) self
    // dependencies before they get here, but the graph must still catch them
    #[test]
    fn errors_when_a_task_depends_on_itself() {
        assert_eq!(
            build_cycle_error(vec![create_task(
                "a",
                vec![create_dep("a", TaskDependencyType::Required)]
            )]),
            "Unable to create task graph, adding a relationship from proj:a to proj:a would introduce a cycle."
        );

        assert_eq!(
            build_cycle_error(vec![create_task(
                "a",
                vec![create_dep("a", TaskDependencyType::Cleanup)]
            )]),
            "Unable to create task graph, adding a relationship from proj:a to proj:a would introduce a cycle."
        );
    }

    #[test]
    fn errors_with_reversed_endpoints_when_a_cleanup_dep_closes_a_cycle() {
        // `a` requires `b` requires `c`, and `c` cleans up after `a`, so the
        // cleanup edge points from `c` (the dependency) to `a` (the task)
        assert_eq!(
            build_cycle_error(vec![
                create_task("b", vec![create_dep("c", TaskDependencyType::Required)]),
                create_task(
                    "a",
                    vec![
                        create_dep("b", TaskDependencyType::Required),
                        create_dep("c", TaskDependencyType::Cleanup),
                    ]
                ),
            ]),
            "Unable to create task graph, adding a relationship from proj:c to proj:a would introduce a cycle."
        );
    }

    #[test]
    fn errors_when_cycle_spans_multiple_builds() {
        let mut builder = WorkspaceTasksBuilder::new();

        builder
            .build(vec![create_task(
                "a",
                vec![create_dep("b", TaskDependencyType::Required)],
            )])
            .unwrap();

        let error = builder
            .build(vec![create_task(
                "b",
                vec![create_dep("a", TaskDependencyType::Required)],
            )])
            .unwrap_err();

        assert_eq!(
            error.to_string(),
            "Unable to create task graph, adding a relationship from proj:b to proj:a would introduce a cycle."
        );
    }

    #[test]
    fn doesnt_error_for_deep_chains() {
        let tasks = (0..5000)
            .map(|i| {
                create_task(
                    &format!("t{i}"),
                    if i == 0 {
                        vec![]
                    } else {
                        vec![create_dep(
                            &format!("t{}", i - 1),
                            TaskDependencyType::Required,
                        )]
                    },
                )
            })
            .rev()
            .collect::<Vec<_>>();

        let mut builder = WorkspaceTasksBuilder::new();
        builder.build(tasks).unwrap();

        assert_eq!(builder.graph.node_count(), 5000);
        assert_eq!(builder.graph.edge_count(), 4999);
    }

    // The cached graph is serialized as a plain petgraph graph (the format
    // `daggy::Dag` also serialized as), so the cache format is unchanged
    #[test]
    fn serializes_graph_for_the_cache() {
        let mut builder = WorkspaceTasksBuilder::new();

        builder
            .build(vec![
                create_task(
                    "build",
                    vec![
                        create_dep("lint", TaskDependencyType::Required),
                        create_dep("clean", TaskDependencyType::Cleanup),
                    ],
                ),
                create_task("lint", vec![]),
            ])
            .unwrap();

        let json = serde_json::to_value(&builder).unwrap();
        let graph = json.get("graph").unwrap().as_object().unwrap();

        assert_eq!(
            graph.keys().collect::<Vec<_>>(),
            ["nodes", "node_holes", "edge_property", "edges"]
        );
        assert_eq!(graph["edge_property"], "directed");
        assert_eq!(graph["node_holes"], serde_json::json!([]));
        assert_eq!(
            graph["edges"],
            serde_json::json!([[0, 1, "required"], [2, 0, "cleanup"]])
        );
        // Never built, so a placeholder
        assert_eq!(graph["nodes"][2], serde_json::Value::Null);
        assert_eq!(graph["nodes"][0]["target"], "proj:build");

        let restored: WorkspaceTasksBuilder = serde_json::from_value(json.clone()).unwrap();

        assert_eq!(restored.graph.node_count(), 3);
        assert_eq!(restored.graph.edge_count(), 2);
        assert_eq!(serde_json::to_value(&restored).unwrap(), json);
    }
}
