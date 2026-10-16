//! Differential tests between the current builder (`ActionGraphBuilder`) and the new,
//! request-based builder (`builder::ActionGraphBuilder`). Both build the same requests,
//! with the same options, and must produce the same canonical graph (see
//! `utils::CanonicalActionGraph`), except where a decision in
//! `docs/todo/ACTION_GRAPH_BUILDER.md` says otherwise.

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
use moon_project::Project;
use moon_task::{Target, TargetLocator, Task};
use moon_test_utils::{WorkspaceMockOptions, WorkspaceMocker};
use moon_toolchain::ToolchainSpec;
use moon_workspace_graph::projects::ScopePartition;
use moon_workspace_graph::{GraphConnections, WorkspaceGraph};
use rustc_hash::FxHashSet;
use starbase_sandbox::create_sandbox;
use std::collections::BTreeSet;
use std::env;
use std::sync::Arc;
use utils::CanonicalActionGraph;

#[derive(Clone, Debug)]
enum Request {
    InstallDependenciesByProject(Arc<Project>),
    RunTasks(Vec<TargetLocator>),
    RunTasksWithPlan(TargetsBlock),
    RunTask(Arc<Task>),
    RunTaskByTarget(Target),
    SetupProto,
    /// Setup the toolchain, and when it inserted a node, its environment in the
    /// workspace root, like `moon setup`.
    SetupToolchainAndEnvironment(ToolchainSpec),
    SyncProject(Arc<Project>),
    SyncWorkspace,
}

/// The builder options, modeled on the commands.
#[derive(Clone, Copy, Debug, Default)]
enum Profile {
    /// Every action (`moon run`, `moon ci`, and the pipeline defaults).
    #[default]
    All,
    /// No actions, only tasks (`--no-actions`).
    None,
    /// Without workspace sync, project dependency sync, or dependency installs.
    ActionsDisabled,
    /// Only toolchain and environment setup (`moon setup`).
    Setup,
    /// Only project syncs, with their dependencies (`moon sync projects`).
    SyncProjects,
}

impl Profile {
    fn options(self) -> ActionGraphBuilderOptions {
        match self {
            Self::All => ActionGraphBuilderOptions::default(),
            Self::None => ActionGraphBuilderOptions::new(false),
            Self::ActionsDisabled => ActionGraphBuilderOptions {
                install_dependencies: false.into(),
                sync_project_dependencies: false,
                sync_workspace: false,
                ..Default::default()
            },
            Self::Setup => ActionGraphBuilderOptions {
                install_dependencies: false.into(),
                setup_environment: true.into(),
                setup_toolchains: true.into(),
                sync_projects: false.into(),
                sync_project_dependencies: false,
                sync_workspace: false,
            },
            Self::SyncProjects => ActionGraphBuilderOptions {
                install_dependencies: false.into(),
                setup_environment: false.into(),
                setup_toolchains: false.into(),
                sync_projects: true.into(),
                sync_project_dependencies: true,
                sync_workspace: false,
            },
        }
    }
}

/// A set of requests, built by both builders.
#[derive(Clone, Debug, Default)]
struct Case {
    name: String,
    options: Profile,
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
                case.options.options(),
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
                    Request::InstallDependenciesByProject(project) => {
                        builder.install_dependencies_by_project(project).await?;
                    }
                    Request::SetupProto => {
                        builder.setup_proto().await?;
                    }
                    Request::SetupToolchainAndEnvironment(spec) => {
                        if builder.setup_toolchain(spec, None).await?.is_some() {
                            builder.setup_environment_root(spec).await?;
                        }
                    }
                    Request::SyncProject(project) => {
                        builder.sync_project(project, &case.reqs).await?;
                    }
                    Request::SyncWorkspace => {
                        builder.sync_workspace().await?;
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
                case.options.options(),
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
                    Request::InstallDependenciesByProject(project) => {
                        builder.install_dependencies_by_project(project);
                    }
                    Request::SetupProto => {
                        builder.setup_proto();
                    }
                    Request::SetupToolchainAndEnvironment(spec) => {
                        builder.setup_toolchain_and_environment(spec);
                    }
                    Request::SyncProject(project) => {
                        builder.sync_project(project);
                    }
                    Request::SyncWorkspace => {
                        builder.sync_workspace();
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

    /// Check the edges between project syncs that differ between the builders,
    /// and remove them from both forms. Project sync edges are linked from the
    /// full set of synced projects (decision 1), so the new builder may have
    /// edges that the current one doesn't, which must be direct dependencies.
    /// Cycles across scopes keep production and peer edges (decision 2), so the
    /// current builder may have edges that the new one doesn't, which must not
    /// be production dependencies, and must close a cycle in the new graph.
    fn check_sync_project_edges(
        &self,
        current: &mut CanonicalActionGraph,
        new: &mut CanonicalActionGraph,
    ) -> Vec<String> {
        fn get_project_id(key: &str) -> Option<&str> {
            key.strip_prefix("SyncProject(")?.strip_suffix(')')
        }

        fn is_sync_edge(edge: &(String, String, &str)) -> bool {
            get_project_id(&edge.0).is_some() && get_project_id(&edge.1).is_some()
        }

        let current_edges = current
            .edges
            .iter()
            .filter(|edge| is_sync_edge(edge))
            .cloned()
            .collect::<BTreeSet<_>>();
        let new_edges = new
            .edges
            .iter()
            .filter(|edge| is_sync_edge(edge))
            .cloned()
            .collect::<BTreeSet<_>>();
        let mut violations = vec![];

        let get_deps = |id: &str, partition: Option<ScopePartition>| {
            let project = self.workspace_graph.get_project(id).unwrap();

            match partition {
                Some(partition) => self
                    .workspace_graph
                    .projects
                    .partitioned_dependencies_of(&project, partition),
                None => self.workspace_graph.projects.dependencies_of(&project),
            }
        };

        for edge in new_edges.difference(&current_edges) {
            let (from, to) = (
                get_project_id(&edge.0).unwrap(),
                get_project_id(&edge.1).unwrap(),
            );

            if !get_deps(from, None).iter().any(|id| id == to) {
                violations.push(format!(
                    "! new edge {} -> {} isn't a project dependency",
                    edge.0, edge.1
                ));
            }
        }

        for edge in current_edges.difference(&new_edges) {
            let (from, to) = (
                get_project_id(&edge.0).unwrap(),
                get_project_id(&edge.1).unwrap(),
            );

            if get_deps(from, Some(ScopePartition::Production))
                .iter()
                .any(|id| id == to)
            {
                violations.push(format!(
                    "! production edge {} -> {} is missing",
                    edge.0, edge.1
                ));
            }

            // Whether `to` reaches `from` through the new graph's sync edges
            let mut queue = vec![edge.1.clone()];
            let mut visited = BTreeSet::new();
            let mut closes_cycle = false;

            while let Some(key) = queue.pop() {
                if key == edge.0 {
                    closes_cycle = true;
                    break;
                }

                if visited.insert(key.clone()) {
                    queue.extend(
                        new_edges
                            .iter()
                            .filter(|next| next.0 == key)
                            .map(|next| next.1.clone()),
                    );
                }
            }

            if !closes_cycle {
                violations.push(format!(
                    "! edge {} -> {} is missing, but doesn't close a cycle",
                    edge.0, edge.1
                ));
            }
        }

        current.edges.retain(|edge| !is_sync_edge(edge));
        new.edges.retain(|edge| !is_sync_edge(edge));

        violations
    }

    /// Build the case with both builders, and describe how they differ.
    async fn compare(&self, case: &Case) -> Option<String> {
        let current = self.build_current(case).await;
        let new = self.build_new(case).await;

        match (current, new) {
            (Ok(mut current), Ok(mut new)) => {
                // Project sync edges differ by design (decisions 1 and 2),
                // so they're checked against the decisions instead
                let violations = self.check_sync_project_edges(&mut current, &mut new);
                let diff = current.diff(&new);

                if diff.is_empty() && violations.is_empty() {
                    None
                } else {
                    Some(format!(
                        "{}: graphs differ (- current, + new):\n{}",
                        case.name,
                        diff.into_iter()
                            .chain(violations)
                            .collect::<Vec<_>>()
                            .join("\n")
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
/// The requests that every workspace is built with: the task requests with
/// every action, and without any action, then the provisioning and sync
/// requests of the other commands.
fn create_cases(prefix: &str, graphs: &Graphs, changed_files: &[String], seed: u64) -> Vec<Case> {
    let mut cases = vec![];

    for case in create_run_cases(prefix, &graphs.workspace_graph, changed_files, seed) {
        cases.push(Case {
            name: format!("{}, no actions", case.name),
            options: Profile::None,
            ..case.clone()
        });
        cases.push(case);
    }

    cases.extend(create_provision_cases(prefix, graphs));
    cases
}

fn create_provision_cases(prefix: &str, graphs: &Graphs) -> Vec<Case> {
    let workspace_graph = &graphs.workspace_graph;
    let mut projects = workspace_graph.get_projects().unwrap();
    let mut tasks = workspace_graph.get_tasks().unwrap();

    projects.sort_by(|a, b| a.id.cmp(&b.id));
    tasks.sort_by(|a, b| a.target.cmp(&b.target));

    let locators = tasks
        .iter()
        .map(|task| TargetLocator::Qualified(task.target.clone()))
        .collect::<Vec<_>>();

    let case = |name: &str, options: Profile, reqs: RunRequirements, requests: Vec<Request>| Case {
        name: format!("{prefix}, {name}"),
        options,
        reqs,
        requests,
        ..Default::default()
    };

    // The same specs that `moon setup` resolves
    let spec_builder = NewActionGraphBuilder::new(
        Arc::clone(&graphs.app_context),
        Arc::clone(workspace_graph),
        ActionGraphBuilderOptions::default(),
    );
    let specs = graphs
        .app_context
        .toolchains_config
        .plugins
        .keys()
        .filter_map(|id| spec_builder.get_workspace_spec(id))
        .collect::<Vec<_>>();

    let mut sync_requests = vec![Request::SyncWorkspace];

    for project in &projects {
        sync_requests.push(Request::SyncProject(Arc::clone(project)));

        for task in &tasks {
            if task.options.run_in_sync_phase
                && task.target.get_project_id().unwrap() == project.id.as_str()
            {
                sync_requests.push(Request::RunTask(Arc::clone(task)));
            }
        }
    }

    vec![
        // moon ci, which syncs the workspace first
        case(
            "ci with workspace sync",
            Profile::All,
            ci(),
            vec![Request::SyncWorkspace, Request::RunTasks(locators.clone())],
        ),
        // moon run, with the pipeline's actions disabled in config
        case(
            "run with actions disabled",
            Profile::ActionsDisabled,
            local(),
            vec![Request::RunTasks(locators)],
        ),
        // moon sync
        case("sync", Profile::All, local(), sync_requests),
        // moon sync projects
        case(
            "sync projects",
            Profile::SyncProjects,
            local(),
            projects
                .iter()
                .map(|project| Request::SyncProject(Arc::clone(project)))
                .collect(),
        ),
        // moon setup
        case(
            "setup",
            Profile::Setup,
            local(),
            [Request::SetupProto]
                .into_iter()
                .chain(specs.into_iter().map(Request::SetupToolchainAndEnvironment))
                .collect(),
        ),
        // moon docker setup, with every project focused
        case(
            "docker setup",
            Profile::All,
            local(),
            projects
                .iter()
                .map(|project| Request::InstallDependenciesByProject(Arc::clone(project)))
                .collect(),
        ),
    ]
}

fn create_run_cases(
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
            &graphs,
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

            for case in create_cases(fixture, &graphs, &[], 0) {
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

        let app_context = Arc::new(mocker.mock_app_context());

        // Without actions, only the tasks are walked
        let mut builder = NewActionGraphBuilder::new(
            Arc::clone(&app_context),
            Arc::clone(&workspace_graph),
            ActionGraphBuilderOptions::new(false),
        );

        builder.run_tasks(&locators);

        let output = builder.build().await.unwrap();

        assert_eq!(output.graph.get_node_count(), locators.len());

        // With actions, every project's sync, and its dependencies, are walked too
        let mut builder = NewActionGraphBuilder::new(
            app_context,
            Arc::clone(&workspace_graph),
            ActionGraphBuilderOptions::default(),
        );

        builder.run_tasks(&locators);

        let output = builder.build().await.unwrap();

        assert!(output.graph.get_node_count() > locators.len() * 2);
    }
}
