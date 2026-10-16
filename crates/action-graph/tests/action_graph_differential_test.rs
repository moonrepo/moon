//! Differential tests between the current builder (`ActionGraphBuilder`) and the new,
//! request-based builder (`builder::ActionGraphBuilder`). Both build the same requests,
//! and must produce the same canonical graph (see `utils::CanonicalActionGraph`).
//!
//! The new builder doesn't resolve provisioning or sync actions yet, so the current
//! builder is built with every option disabled, which produces the task subgraph.

mod utils;

use moon_action_graph::builder::ActionGraphBuilder as NewActionGraphBuilder;
use moon_action_graph::{
    ActionGraphBuilder, ActionGraphBuilderOptions, RunPartition, RunRequirements,
};
use moon_affected::Depth;
use moon_app_context::AppContext;
use moon_bench_utils::{create_chained_workspace, create_generated_workspace};
use moon_common::path::WorkspaceRelativePathBuf;
use moon_exec_plan::{ExecutionPlan, TargetsBlock};
use moon_task::{Target, TargetLocator, Task};
use moon_test_utils::{WorkspaceMockOptions, WorkspaceMocker};
use moon_workspace_graph::WorkspaceGraph;
use rustc_hash::FxHashSet;
use starbase_sandbox::create_sandbox;
use std::env;
use std::sync::Arc;
use utils::CanonicalActionGraph;

// Provisioning and sync requests, without the prefix, are added next
#[allow(clippy::enum_variant_names)]
#[derive(Clone, Debug)]
enum Request {
    RunTasks(Vec<TargetLocator>),
    RunTasksWithPlan(TargetsBlock),
    RunTask(Arc<Task>),
    RunTaskByTarget(Target),
}

/// A set of requests, built by both builders.
#[derive(Clone, Debug, Default)]
struct Case {
    name: String,
    reqs: RunRequirements,
    affected: Option<(Depth, bool)>,
    changed_files: Vec<String>,
    query: Option<String>,
    requests: Vec<Request>,
}

struct Graphs {
    app_context: Arc<AppContext>,
    workspace_graph: Arc<WorkspaceGraph>,
}

fn canonicalize(
    context: &moon_action_context::ActionContext,
    graph: &moon_action_graph::ActionGraph,
    partition: Option<&RunPartition>,
) -> CanonicalActionGraph {
    let canonical = CanonicalActionGraph::new(context, graph);

    match partition {
        Some(partition) => canonical.with_partition(graph, partition),
        None => canonical,
    }
}

fn get_changed_files(case: &Case) -> FxHashSet<WorkspaceRelativePathBuf> {
    case.changed_files
        .iter()
        .map(WorkspaceRelativePathBuf::from)
        .collect()
}

impl Graphs {
    async fn build_current(&self, case: &Case) -> Result<CanonicalActionGraph, String> {
        let result: miette::Result<CanonicalActionGraph> = async {
            let mut builder = ActionGraphBuilder::new(
                Arc::clone(&self.app_context),
                Arc::clone(&self.workspace_graph),
                ActionGraphBuilderOptions::new(false),
            )?;

            builder.set_changed_files(get_changed_files(case))?;

            if let Some(query) = &case.query {
                builder.set_query(query)?;
            }

            if let Some((depth, ci_check)) = case.affected {
                builder.track_affected(depth, ci_check).await?;
            }

            let mut partition = None;

            for request in &case.requests {
                match request {
                    Request::RunTasks(locators) => {
                        let result = builder.run_tasks(locators, case.reqs.clone()).await?;
                        partition.get_or_insert(result);
                    }
                    Request::RunTasksWithPlan(targets) => {
                        let plan = ExecutionPlan {
                            targets: targets.clone(),
                            ..Default::default()
                        };
                        let result = builder
                            .run_tasks_with_plan(&plan, case.reqs.clone())
                            .await?;
                        partition.get_or_insert(result);
                    }
                    Request::RunTask(task) => {
                        builder.run_task(task, &case.reqs).await?;
                    }
                    Request::RunTaskByTarget(target) => {
                        builder.run_task_by_target(target, &case.reqs).await?;
                    }
                };
            }

            let (context, graph) = builder.build();

            Ok(canonicalize(&context, &graph, partition.as_ref()))
        }
        .await;

        result.map_err(|error| error.to_string())
    }

    async fn build_new(&self, case: &Case) -> Result<CanonicalActionGraph, String> {
        let result: miette::Result<CanonicalActionGraph> = async {
            let mut builder = NewActionGraphBuilder::new(
                Arc::clone(&self.app_context),
                Arc::clone(&self.workspace_graph),
                ActionGraphBuilderOptions::new(false),
            );

            builder.set_requirements(case.reqs.clone());
            builder.set_changed_files(get_changed_files(case));

            if let Some(query) = &case.query {
                builder.set_query(query)?;
            }

            if let Some((depth, ci_check)) = case.affected {
                builder.track_affected(depth, ci_check);
            }

            let mut partition_id = None;

            for request in &case.requests {
                match request {
                    Request::RunTasks(locators) => {
                        let id = builder.run_tasks(locators);
                        partition_id.get_or_insert(id);
                    }
                    Request::RunTasksWithPlan(targets) => {
                        let id = builder.run_tasks_with_plan(&ExecutionPlan {
                            targets: targets.clone(),
                            ..Default::default()
                        });
                        partition_id.get_or_insert(id);
                    }
                    Request::RunTask(task) => {
                        builder.run_task(task);
                    }
                    Request::RunTaskByTarget(target) => {
                        builder.run_task_by_target(target);
                    }
                };
            }

            let output = builder.build().await?;
            let partition = partition_id.and_then(|id| output.report.partition(id));

            Ok(canonicalize(&output.context, &output.graph, partition))
        }
        .await;

        result.map_err(|error| error.to_string())
    }

    /// Build the case with both builders, and describe how they differ.
    async fn compare(&self, case: &Case) -> Option<String> {
        let current = self.build_current(case).await;
        let new = self.build_new(case).await;

        match (current, new) {
            (Ok(current), Ok(new)) => {
                let diff = current.diff(&new);

                if diff.is_empty() {
                    None
                } else {
                    Some(format!(
                        "{}: graphs differ (- current, + new):\n{}",
                        case.name,
                        diff.join("\n")
                    ))
                }
            }
            (Err(current), Err(new)) => {
                if current == new {
                    None
                } else {
                    Some(format!(
                        "{}: errors differ:\n- {current}\n+ {new}",
                        case.name
                    ))
                }
            }
            (Ok(_), Err(new)) => Some(format!("{}: only the new builder failed: {new}", case.name)),
            (Err(current), Ok(_)) => Some(format!(
                "{}: only the current builder failed: {current}",
                case.name
            )),
        }
    }
}

fn local() -> RunRequirements {
    RunRequirements {
        ci: false,
        ..Default::default()
    }
}

fn ci() -> RunRequirements {
    RunRequirements {
        ci: true,
        ci_check: true,
        ..Default::default()
    }
}

/// The requests that every workspace is built with, modeled on the commands.
fn create_cases(
    prefix: &str,
    workspace_graph: &WorkspaceGraph,
    changed_files: &[String],
    seed: u64,
) -> Vec<Case> {
    let mut tasks = workspace_graph.get_tasks().unwrap();

    tasks.sort_by(|a, b| a.target.cmp(&b.target));

    let locators = tasks
        .iter()
        .map(|task| TargetLocator::Qualified(task.target.clone()))
        .collect::<Vec<_>>();
    let every_other = locators.iter().step_by(2).cloned().collect::<Vec<_>>();
    let job = seed as usize % 2;

    let case = |name: &str, reqs: RunRequirements, requests: Vec<Request>| Case {
        name: format!("{prefix}, {name}"),
        reqs,
        changed_files: changed_files.to_vec(),
        requests,
        ..Default::default()
    };

    vec![
        // moon run
        case("run", local(), vec![Request::RunTasks(locators.clone())]),
        // moon ci
        case("ci", ci(), vec![Request::RunTasks(locators.clone())]),
        // moon run --affected
        Case {
            affected: Some((Depth::NONE, false)),
            ..case(
                "run affected",
                local(),
                vec![Request::RunTasks(locators.clone())],
            )
        },
        // moon ci --affected-depth deep --dependents deep
        Case {
            affected: Some((Depth::Deep, true)),
            ..case(
                "ci affected deep",
                RunRequirements {
                    dependents: Depth::Deep,
                    ..ci()
                },
                vec![Request::RunTasks(locators.clone())],
            )
        },
        // moon ci --job <n> --job-total 2
        Case {
            affected: Some((Depth::NONE, true)),
            ..case(
                "ci partitioned",
                RunRequirements {
                    job: Some(job),
                    job_total: Some(2),
                    ..ci()
                },
                vec![Request::RunTasks(locators.clone())],
            )
        },
        // moon ci --plan <file> with targets partitioned across 2 jobs
        case(
            "ci plan partitioned",
            RunRequirements {
                job: Some(job),
                job_total: Some(2),
                ..ci()
            },
            vec![Request::RunTasksWithPlan(TargetsBlock::Partitioned {
                jobs: vec![
                    every_other.clone(),
                    locators.iter().skip(1).step_by(2).cloned().collect(),
                ],
            })],
        ),
        // moon check --interactive
        case(
            "check interactive",
            RunRequirements {
                interactive: true,
                skip_persistent: true,
                ..local()
            },
            vec![Request::RunTasks(locators.clone())],
        ),
        // moon run --no-deps with every other target
        case(
            "run without dependencies",
            RunRequirements {
                no_dependencies: true,
                ..local()
            },
            vec![Request::RunTasks(every_other.clone())],
        ),
        // moon run :t0 :t1 --query 'project~p*'
        Case {
            query: Some("project~p*".into()),
            ..case(
                "run with query",
                local(),
                vec![Request::RunTasks(vec![
                    TargetLocator::parse(":t0").unwrap(),
                    TargetLocator::parse(":t1").unwrap(),
                ])],
            )
        },
        // moon action-graph
        case(
            "action-graph",
            local(),
            tasks
                .iter()
                .map(|task| Request::RunTask(Arc::clone(task)))
                .collect(),
        ),
        // moon action-graph <targets> --dependents direct
        case(
            "action-graph targets",
            RunRequirements {
                dependents: Depth::DIRECT,
                ..local()
            },
            tasks
                .iter()
                .map(|task| Request::RunTaskByTarget(task.target.clone()))
                .collect(),
        ),
    ]
}

/// The seeds to generate workspaces for: `ACTION_GRAPH_SEED` for a single
/// seed (to reproduce a failure), or the first `ACTION_GRAPH_SEEDS` seeds.
fn get_seeds(shard: u64, shards: u64) -> Vec<u64> {
    if let Ok(seed) = env::var("ACTION_GRAPH_SEED") {
        let seed = seed.parse::<u64>().unwrap();

        return if seed % shards == shard {
            vec![seed]
        } else {
            vec![]
        };
    }

    let count = env::var("ACTION_GRAPH_SEEDS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(64);

    (0..count).filter(|seed| seed % shards == shard).collect()
}

async fn compare_seeds(shard: u64, shards: u64) {
    let mut failures = vec![];

    for seed in get_seeds(shard, shards) {
        // Printed so that a panic can be traced back to its seed
        eprintln!("Comparing seed {seed}");

        let workspace = create_generated_workspace(seed);
        let mocker = WorkspaceMocker::new(workspace.sandbox.path())
            .load_default_configs()
            .set_working_dir(workspace.working_dir.clone());

        // Share the plugin registries, as loading plugins is slow
        let app_context = Arc::new(mocker.mock_app_context());
        let mut context = mocker.mock_workspace_builder_context();
        context.extension_registry = Arc::clone(&app_context.extension_registry);
        context.toolchain_registry = Arc::clone(&app_context.toolchain_registry);

        let workspace_graph = mocker
            .mock_workspace_graph_with_options(WorkspaceMockOptions {
                context: Some(context),
                ..Default::default()
            })
            .await;

        let graphs = Graphs {
            app_context,
            workspace_graph: Arc::new(workspace_graph),
        };

        for case in create_cases(
            &format!("seed {seed}"),
            &graphs.workspace_graph,
            &workspace.changed_files,
            seed,
        ) {
            if let Some(failure) = graphs.compare(&case).await {
                failures.push(failure);
            }
        }
    }

    assert!(
        failures.is_empty(),
        "{} differences:\n{}",
        failures.len(),
        failures.join("\n\n")
    );
}

mod generated_workspaces {
    use super::*;

    // Sharded so that the seeds are compared in parallel by nextest

    #[tokio::test(flavor = "multi_thread")]
    async fn match_current_builder_shard_0() {
        compare_seeds(0, 4).await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn match_current_builder_shard_1() {
        compare_seeds(1, 4).await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn match_current_builder_shard_2() {
        compare_seeds(2, 4).await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn match_current_builder_shard_3() {
        compare_seeds(3, 4).await;
    }
}

mod fixtures {
    use super::*;

    // `tasks-ci-mismatch` is left out, as its workspace graph fails to load by design
    const FIXTURES: [&str; 14] = [
        "affected-starve",
        "dep-scopes",
        "dep-types",
        "dep-workspace",
        "dependent-scopes",
        "graph-depth",
        "persistent-ci",
        "projects",
        "serial-conflict",
        "serial-cycle",
        "serial-persistent",
        "serial-shared",
        "serial-subtree",
        "tasks",
    ];

    #[tokio::test(flavor = "multi_thread")]
    async fn match_current_builder() {
        let mut failures = vec![];

        for fixture in FIXTURES {
            let sandbox = create_sandbox(fixture);
            let container = utils::ActionGraphContainer::new(sandbox.path());
            let graphs = Graphs {
                app_context: Arc::new(container.mocker.mock_app_context()),
                workspace_graph: container.create_workspace_graph().await,
            };

            for case in create_cases(fixture, &graphs.workspace_graph, &[], 0) {
                if let Some(failure) = graphs.compare(&case).await {
                    failures.push(failure);
                }
            }
        }

        assert!(
            failures.is_empty(),
            "{} differences:\n{}",
            failures.len(),
            failures.join("\n\n")
        );
    }
}

mod request_orders {
    use super::*;

    // Serial ordering conflicts depend on the order that tasks are requested
    // in (see the `serial_conflicts` tests), so request every pair of tasks,
    // in both orders, and every task in reverse order
    #[tokio::test(flavor = "multi_thread")]
    async fn match_current_builder_for_serial_fixtures() {
        let mut failures = vec![];

        for fixture in [
            "serial-conflict",
            "serial-cycle",
            "serial-persistent",
            "serial-shared",
            "serial-subtree",
            "dep-types",
        ] {
            let sandbox = create_sandbox(fixture);
            let container = utils::ActionGraphContainer::new(sandbox.path());
            let graphs = Graphs {
                app_context: Arc::new(container.mocker.mock_app_context()),
                workspace_graph: container.create_workspace_graph().await,
            };

            let mut tasks = graphs.workspace_graph.get_tasks().unwrap();
            tasks.sort_by(|a, b| a.target.cmp(&b.target));

            let mut orders = vec![tasks.iter().rev().cloned().collect::<Vec<_>>()];

            for a in &tasks {
                for b in &tasks {
                    if a.target != b.target {
                        orders.push(vec![Arc::clone(a), Arc::clone(b)]);
                    }
                }
            }

            for order in orders {
                for reqs in [
                    local(),
                    RunRequirements {
                        no_dependencies: true,
                        ..local()
                    },
                ] {
                    let case = Case {
                        name: format!(
                            "{fixture}, {}{}",
                            order
                                .iter()
                                .map(|task| task.target.as_str())
                                .collect::<Vec<_>>()
                                .join(" then "),
                            if reqs.no_dependencies {
                                ", without dependencies"
                            } else {
                                ""
                            },
                        ),
                        reqs,
                        requests: order
                            .iter()
                            .map(|task| Request::RunTask(Arc::clone(task)))
                            .collect(),
                        ..Default::default()
                    };

                    if let Some(failure) = graphs.compare(&case).await {
                        failures.push(failure);
                    }
                }
            }
        }

        assert!(
            failures.is_empty(),
            "{} differences:\n{}",
            failures.len(),
            failures.join("\n\n")
        );
    }
}

mod errors {
    use super::*;
    use moon_config::{TaskDependencyConfig, TaskDependencyType};

    // The message is the same as the current builder's, while the help
    // describes the whole cycle (decision 8)
    #[tokio::test(flavor = "multi_thread")]
    async fn describes_the_cycle() {
        let sandbox = create_sandbox("dep-types");
        let container = utils::ActionGraphContainer::new(sandbox.path());
        let workspace_graph = container.create_workspace_graph().await;

        // parent -> mid -> cleanup, and then cleanup -> parent
        let mut task = workspace_graph
            .get_task_from_project("proj", "cycle-parent")
            .unwrap()
            .as_ref()
            .to_owned();

        task.deps.push(TaskDependencyConfig {
            target: Target::parse("proj:cycle-cleanup").unwrap(),
            type_of: TaskDependencyType::Cleanup,
            ..TaskDependencyConfig::default()
        });

        let mut builder = NewActionGraphBuilder::new(
            Arc::new(container.mocker.mock_app_context()),
            workspace_graph,
            ActionGraphBuilderOptions::new(false),
        );

        builder.run_task(&task);

        let error = builder.build().await.err().unwrap();

        assert_eq!(
            error.to_string(),
            "Unable to create action graph, adding a relationship from action RunTask(proj:cycle-cleanup) to RunTask(proj:cycle-parent) would introduce a cycle."
        );
        assert_eq!(
            error.help().map(|help| help.to_string()).as_deref(),
            Some(
                "The cycle is RunTask(proj:cycle-cleanup) → RunTask(proj:cycle-parent) → RunTask(proj:cycle-mid) → RunTask(proj:cycle-cleanup)"
            )
        );
    }
}

mod deep_chains {
    use super::*;

    // The current builder recurses once per dependency, and overflows the stack
    // for a chain of 1000 projects, while the new builder walks dependencies
    // with an explicit stack. Tests run on a thread with a 2MB stack
    #[tokio::test(flavor = "multi_thread")]
    async fn doesnt_overflow_the_stack() {
        let sandbox = create_chained_workspace(1000);
        let mocker = WorkspaceMocker::new(sandbox.path()).load_default_configs();
        let workspace_graph = Arc::new(mocker.mock_workspace_graph().await);

        let locators = workspace_graph
            .get_tasks()
            .unwrap()
            .into_iter()
            .map(|task| TargetLocator::Qualified(task.target.clone()))
            .collect::<Vec<_>>();

        let mut builder = NewActionGraphBuilder::new(
            Arc::new(mocker.mock_app_context()),
            Arc::clone(&workspace_graph),
            ActionGraphBuilderOptions::new(false),
        );

        builder.run_tasks(&locators);

        let output = builder.build().await.unwrap();

        assert_eq!(output.graph.get_node_count(), locators.len());
    }
}
