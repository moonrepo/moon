mod utils;

use moon_action_context::ActionContext;
use moon_action_graph::{
    ActionGraph, ActionGraphBuilder, ActionGraphBuilderOptions, BuildOutput, RunPartition,
    RunRequirements,
};
use moon_affected::Depth;
use moon_app_context::AppContext;
use moon_bench_utils::{GeneratedWorkspace, create_generated_workspace};
use moon_common::path::WorkspaceRelativePathBuf;
use moon_task::TargetLocator;
use moon_test_utils::{WorkspaceMockOptions, WorkspaceMocker};
use moon_workspace_graph::WorkspaceGraph;
use starbase_sandbox::create_sandbox;
use std::collections::BTreeSet;
use std::env;
use std::sync::Arc;
use utils::{
    ActionGraphContainer, CanonicalActionGraph, assert_canonical_eq, find_invariant_violations,
};

/// How a graph is built for a generated workspace, modeled on the commands.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    /// `moon run` with every target.
    RunTasks,
    /// `moon ci` with every target: in CI, checking `runInCI`, and syncing
    /// the workspace first.
    RunTasksInCi,
    /// `moon run --affected` with every target.
    RunTasksAffected,
    /// `moon ci --affected-depth deep --dependents deep` with every target.
    RunTasksAffectedDeep,
    /// `moon ci --job <n> --job-total 2` with every target.
    RunTasksPartitioned,
    /// `moon check --interactive` with every target, which skips persistent tasks.
    RunTasksSkipPersistent,
    /// `moon run --no-deps` with every other target.
    RunTasksWithoutDependencies,
    /// `moon run :t0 :t1 --query 'project~p*'`.
    RunTasksWithQuery,
    /// Every target, with workspace sync, project dependency sync, and
    /// dependency installs disabled.
    RunTasksWithActionsDisabled,
    /// `moon action-graph` with no targets, which runs every task.
    RunTaskEach,
    /// `moon sync`.
    Sync,
    /// `moon setup`.
    Setup,
    /// `moon docker setup` with every project focused.
    DockerSetup,
}

const SCENARIOS: [Scenario; 13] = [
    Scenario::RunTasks,
    Scenario::RunTasksInCi,
    Scenario::RunTasksAffected,
    Scenario::RunTasksAffectedDeep,
    Scenario::RunTasksPartitioned,
    Scenario::RunTasksSkipPersistent,
    Scenario::RunTasksWithoutDependencies,
    Scenario::RunTasksWithQuery,
    Scenario::RunTasksWithActionsDisabled,
    Scenario::RunTaskEach,
    Scenario::Sync,
    Scenario::Setup,
    Scenario::DockerSetup,
];

struct Build {
    context: ActionContext,
    graph: ActionGraph,
    partition: Option<RunPartition>,
    sync_workspace: bool,
}

struct GeneratedGraphs {
    app_context: Arc<AppContext>,
    workspace: GeneratedWorkspace,
    workspace_graph: Arc<WorkspaceGraph>,
}

impl GeneratedGraphs {
    async fn new(seed: u64) -> Self {
        let workspace = create_generated_workspace(seed);
        let mocker = WorkspaceMocker::new(workspace.sandbox.path())
            .load_default_configs()
            .set_working_dir(workspace.working_dir.clone());

        // Share the plugin registries between both contexts, so that plugins are
        // only loaded and dropped once per seed, as dropping compiled plugins is
        // slow on macOS (wasmtime deregisters the unwind info of every function)
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

        Self {
            app_context,
            workspace_graph: Arc::new(workspace_graph),
            workspace,
        }
    }

    fn locators(&self, step: usize) -> Vec<TargetLocator> {
        self.workspace
            .targets
            .iter()
            .step_by(step)
            .map(|target| TargetLocator::parse(target).unwrap())
            .collect()
    }

    async fn build(&self, scenario: Scenario) -> miette::Result<Build> {
        let options = match scenario {
            Scenario::RunTasksWithActionsDisabled => ActionGraphBuilderOptions {
                install_dependencies: false.into(),
                sync_project_dependencies: false,
                sync_workspace: false,
                ..Default::default()
            },
            Scenario::Setup => ActionGraphBuilderOptions {
                install_dependencies: false.into(),
                setup_environment: true.into(),
                setup_toolchains: true.into(),
                sync_projects: false.into(),
                sync_project_dependencies: false,
                sync_workspace: false,
            },
            _ => ActionGraphBuilderOptions::default(),
        };
        let sync_workspace = options.sync_workspace;

        let mut builder = ActionGraphBuilder::new(
            Arc::clone(&self.app_context),
            Arc::clone(&self.workspace_graph),
            options,
        );

        builder.set_changed_files(
            self.workspace
                .changed_files
                .iter()
                .map(WorkspaceRelativePathBuf::from)
                .collect(),
        );

        let local = RunRequirements {
            ci: false,
            ..Default::default()
        };
        let ci = RunRequirements {
            ci: true,
            ci_check: true,
            ..Default::default()
        };
        let mut partition = None;

        match scenario {
            Scenario::RunTasks | Scenario::RunTasksWithActionsDisabled => {
                builder.set_requirements(local);
                partition = Some(builder.run_tasks(self.locators(1)));
            }
            Scenario::RunTasksInCi => {
                builder.set_requirements(ci);
                builder.sync_workspace();
                partition = Some(builder.run_tasks(self.locators(1)));
            }
            Scenario::RunTasksAffected => {
                builder.set_requirements(local);
                builder.track_affected(Depth::NONE, false);
                partition = Some(builder.run_tasks(self.locators(1)));
            }
            Scenario::RunTasksAffectedDeep => {
                builder.set_requirements(RunRequirements {
                    dependents: Depth::Deep,
                    ..ci
                });
                builder.track_affected(Depth::Deep, true);
                builder.sync_workspace();
                partition = Some(builder.run_tasks(self.locators(1)));
            }
            Scenario::RunTasksPartitioned => {
                builder.set_requirements(RunRequirements {
                    job: Some(self.workspace.seed as usize % 2),
                    job_total: Some(2),
                    ..ci
                });
                builder.track_affected(Depth::NONE, true);
                builder.sync_workspace();
                partition = Some(builder.run_tasks(self.locators(1)));
            }
            Scenario::RunTasksSkipPersistent => {
                builder.set_requirements(RunRequirements {
                    interactive: true,
                    skip_persistent: true,
                    ..local
                });
                partition = Some(builder.run_tasks(self.locators(1)));
            }
            Scenario::RunTasksWithoutDependencies => {
                builder.set_requirements(RunRequirements {
                    no_dependencies: true,
                    ..local
                });
                partition = Some(builder.run_tasks(self.locators(2)));
            }
            Scenario::RunTasksWithQuery => {
                builder.set_requirements(local);
                builder.set_query("project~p*")?;
                partition = Some(
                    builder.run_tasks([TargetLocator::parse(":t0")?, TargetLocator::parse(":t1")?]),
                );
            }
            Scenario::RunTaskEach => {
                builder.set_requirements(local);

                for task in self.workspace_graph.get_tasks()? {
                    builder.run_task(&task);
                }
            }
            Scenario::Sync => {
                builder.set_requirements(local);
                builder.sync_workspace();

                for project in self.workspace_graph.get_projects()? {
                    builder.sync_project(&project);
                }
            }
            Scenario::Setup => {
                builder.setup_proto();

                for toolchain_id in self.app_context.toolchains_config.plugins.keys() {
                    if let Some(spec) = builder.get_workspace_spec(toolchain_id) {
                        builder.setup_toolchain_and_environment(&spec);
                    }
                }
            }
            Scenario::DockerSetup => {
                for project in self.workspace_graph.get_projects()? {
                    builder.install_dependencies_by_project(&project);
                }
            }
        };

        let BuildOutput {
            context,
            graph,
            mut report,
        } = builder.build().await?;

        Ok(Build {
            context,
            graph,
            partition: partition.and_then(|request| report.take_partition(request)),
            sync_workspace,
        })
    }

    fn canonicalize(&self, build: &Build) -> CanonicalActionGraph {
        let canonical = CanonicalActionGraph::new(&build.context, &build.graph);

        match &build.partition {
            Some(partition) => canonical.with_partition(&build.graph, partition),
            None => canonical,
        }
    }
}

/// What the generated workspaces must produce across the seeds of a shard,
/// so that the generator can't silently stop exercising part of the graph.
const COVERAGE: [&str; 17] = [
    "node InstallDependencies",
    "node RunTask",
    "node SetupEnvironment",
    "node SetupProto",
    "node SetupToolchain",
    "node SyncProject",
    "node SyncWorkspace",
    "node with args or env",
    "node with members",
    "edge cleanup",
    "edge wait",
    "edge SetupEnvironment -> SetupEnvironment",
    "edge SetupToolchain -> SetupToolchain",
    "edge SyncProject -> SyncProject",
    "context affected tasks",
    "context ignored dependencies",
    "context target states",
];

/// Coverage is only checked for at least this many seeds per shard, which
/// the default seeds satisfy, as fewer seeds may not produce everything.
const COVERAGE_MIN_SEEDS: usize = 16;

fn get_node_kind(key: &str) -> &str {
    let kind = key.split('(').next().unwrap_or(key);

    if kind.starts_with("Run") {
        "RunTask"
    } else {
        kind
    }
}

fn record_coverage(canonical: &CanonicalActionGraph, covered: &mut BTreeSet<String>) {
    for key in canonical.nodes.keys() {
        covered.insert(format!("node {}", get_node_kind(key)));

        if key.contains(" args=") || key.contains(" env=") {
            covered.insert("node with args or env".into());
        }

        if key.contains(" members=") {
            covered.insert("node with members".into());
        }
    }

    for (from, to, weight) in &canonical.edges {
        covered.insert(format!("edge {weight}"));
        covered.insert(format!(
            "edge {} -> {}",
            get_node_kind(from),
            get_node_kind(to)
        ));
    }

    let context = &canonical.context;

    if context
        .affected
        .as_ref()
        .is_some_and(|affected| !affected.tasks.is_empty())
    {
        covered.insert("context affected tasks".into());
    }

    if !context.ignored_dependencies.is_empty() {
        covered.insert("context ignored dependencies".into());
    }

    if !context.target_states.is_empty() {
        covered.insert("context target states".into());
    }
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

async fn check_seeds(shard: u64, shards: u64) {
    let seeds = get_seeds(shard, shards);
    let mut covered = BTreeSet::default();
    let mut failures = vec![];

    for &seed in &seeds {
        // Printed so that a panic while generating or loading
        // the workspace can be traced back to its seed
        eprintln!("Checking seed {seed}");

        let graphs = GeneratedGraphs::new(seed).await;
        let describe = |scenario: Scenario| {
            format!(
                "seed {seed}, {scenario:?}, working dir {:?}",
                graphs.workspace.working_dir_kind
            )
        };

        for scenario in SCENARIOS {
            match graphs.build(scenario).await {
                Ok(build) => {
                    record_coverage(&graphs.canonicalize(&build), &mut covered);

                    for violation in find_invariant_violations(&build.graph, build.sync_workspace) {
                        failures.push(format!("{}: {violation}", describe(scenario)));
                    }
                }
                Err(error) => {
                    failures.push(format!(
                        "{}: failed to build: {error:?}",
                        describe(scenario)
                    ));
                }
            };
        }

        // Building the same requests again must produce the same graph,
        // otherwise canonical forms can't be compared across builders
        if let (Ok(first), Ok(second)) = (
            graphs.build(Scenario::RunTasksAffectedDeep).await,
            graphs.build(Scenario::RunTasksAffectedDeep).await,
        ) {
            let diff = graphs
                .canonicalize(&first)
                .diff(&graphs.canonicalize(&second));

            if !diff.is_empty() {
                failures.push(format!(
                    "{}: rebuilding is not deterministic:\n{}",
                    describe(Scenario::RunTasksAffectedDeep),
                    diff.join("\n")
                ));
            }
        }
    }

    if seeds.len() >= COVERAGE_MIN_SEEDS {
        for feature in COVERAGE {
            if !covered.contains(feature) {
                failures.push(format!("no generated workspace produced a {feature}"));
            }
        }
    }

    assert!(
        failures.is_empty(),
        "{} invariant violations:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

mod generated_workspaces {
    use super::*;

    // Sharded so that the seeds are checked in parallel by nextest

    #[tokio::test(flavor = "multi_thread")]
    async fn uphold_invariants_shard_0() {
        check_seeds(0, 4).await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn uphold_invariants_shard_1() {
        check_seeds(1, 4).await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn uphold_invariants_shard_2() {
        check_seeds(2, 4).await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn uphold_invariants_shard_3() {
        check_seeds(3, 4).await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn canonical_form_ignores_request_order() {
        let graphs = GeneratedGraphs::new(7).await;
        let canonicalize = |build: Build| CanonicalActionGraph::new(&build.context, &build.graph);

        // Every primary task is inserted by the same requests, in a different
        // order, but setup and sync nodes are reached before or after them
        let mut forward = graphs
            .build(Scenario::RunTasksWithActionsDisabled)
            .await
            .unwrap();
        let mut builder = ActionGraphBuilder::new(
            Arc::clone(&graphs.app_context),
            Arc::clone(&graphs.workspace_graph),
            ActionGraphBuilderOptions {
                install_dependencies: false.into(),
                sync_project_dependencies: false,
                sync_workspace: false,
                ..Default::default()
            },
        );

        builder.set_changed_files(
            graphs
                .workspace
                .changed_files
                .iter()
                .map(WorkspaceRelativePathBuf::from)
                .collect(),
        );

        let mut locators = graphs.locators(1);
        locators.reverse();

        builder.set_requirements(RunRequirements {
            ci: false,
            ..Default::default()
        });
        builder.run_tasks(locators);

        let BuildOutput { context, graph, .. } = builder.build().await.unwrap();

        // Node indices differ, but the graphs are the same
        forward.partition = None;

        assert_canonical_eq(
            &canonicalize(forward),
            &canonicalize(Build {
                context,
                graph,
                partition: None,
                sync_workspace: false,
            }),
        );
    }
}

mod canonical_form {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn distinguishes_dependency_variants() {
        let sandbox = create_sandbox("dep-types");
        let mut container = ActionGraphContainer::new(sandbox.path());

        let wg = container.create_workspace_graph().await;
        let mut builder = container.create_builder(wg.clone()).await;

        // Both depend on the teardown as a cleanup, once with args
        for id in ["base", "args-parent"] {
            builder.set_requirements(RunRequirements::default());
            builder.run_task(&wg.get_task_from_project("proj", id).unwrap());
        }

        let BuildOutput { context, graph, .. } = builder.build().await.unwrap();
        let canonical = CanonicalActionGraph::new(&context, &graph);
        let variant = r#"RunTask(proj:teardown) args=["--force"]"#;

        assert!(canonical.duplicate_nodes.is_empty());
        assert_eq!(
            canonical
                .nodes
                .iter()
                .filter(|(_, node)| node.label == "RunTask(proj:teardown)")
                .map(|(key, _)| key.as_str())
                .collect::<Vec<_>>(),
            vec!["RunTask(proj:teardown)", variant]
        );
        assert_eq!(
            canonical.cleanup_nodes,
            BTreeSet::from_iter(["RunTask(proj:teardown)".to_owned(), variant.to_owned()])
        );
        assert!(canonical.edges.contains(&(
            variant.to_owned(),
            "RunTask(proj:args-parent)".to_owned(),
            "cleanup"
        )));
        assert!(canonical.context.cleanup_targets.contains("proj:teardown"));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn diffs_by_line() {
        let sandbox = create_sandbox("dep-types");
        let mut container = ActionGraphContainer::new(sandbox.path());

        let wg = container.create_workspace_graph().await;
        let mut canonicals = vec![];

        for id in ["base", "waits"] {
            let mut builder = container.create_builder(wg.clone()).await;

            builder.set_requirements(RunRequirements::default());
            builder.run_task(&wg.get_task_from_project("proj", id).unwrap());

            let BuildOutput { context, graph, .. } = builder.build().await.unwrap();

            canonicals.push(CanonicalActionGraph::new(&context, &graph));
        }

        let diff = canonicals[0].diff(&canonicals[1]);

        assert!(
            diff.contains(&"- edge RunTask(proj:base) -> RunTask(proj:setup) (required)".into())
        );
        assert!(diff.contains(
            &"+ edge RunTask(proj:waits) -> RunPersistentTask(proj:server) (wait)".into()
        ));
        assert!(diff.contains(&"+ context.wait_targets proj:server".into()));
        assert!(!diff.iter().any(|line| line.contains("SyncWorkspace (")));
        assert!(canonicals[0].diff(&canonicals[0]).is_empty());
    }
}

mod invariants {
    use super::*;
    use daggy::Dag;
    use moon_action::{ActionNode, RunTaskNode, SetupToolchainNode, SyncProjectNode};
    use moon_common::Id;
    use moon_config::{TaskDependencyType, UnresolvedVersionSpec};
    use moon_task::Target;
    use moon_toolchain::ToolchainSpec;
    use petgraph::graph::NodeIndex;
    use rustc_hash::FxHashSet;

    struct GraphMocker {
        graph: Dag<ActionNode, TaskDependencyType>,
    }

    impl GraphMocker {
        fn add(&mut self, node: ActionNode) -> NodeIndex {
            self.graph.add_node(node)
        }

        fn link(&mut self, from: NodeIndex, to: NodeIndex, weight: TaskDependencyType) {
            self.graph.add_edge(from, to, weight).unwrap();
        }
    }

    fn create_task(id: &str) -> ActionNode {
        ActionNode::run_task(RunTaskNode::new(Target::new("p", id).unwrap()))
    }

    fn create_toolchain(id: &str) -> ActionNode {
        ActionNode::setup_toolchain(SetupToolchainNode {
            toolchain: ToolchainSpec::new(
                Id::raw(id),
                UnresolvedVersionSpec::parse("1.2.3").unwrap(),
            ),
        })
    }

    #[test]
    fn reports_each_violation() {
        let mut mocker = GraphMocker { graph: Dag::new() };

        let sync = mocker.add(ActionNode::sync_workspace());
        let task_a = mocker.add(create_task("a"));
        let task_b = mocker.add(create_task("b"));
        let tool = mocker.add(create_toolchain("tc-tier3"));
        let tool_wait = mocker.add(create_toolchain("tc-tier2"));
        mocker.add(ActionNode::sync_project(SyncProjectNode {
            project_id: Id::raw("p"),
        }));
        mocker.add(create_task("a"));

        mocker.link(task_a, sync, TaskDependencyType::Required);
        mocker.link(task_b, task_a, TaskDependencyType::Optional);
        mocker.link(tool, task_a, TaskDependencyType::Required);
        mocker.link(tool_wait, sync, TaskDependencyType::Wait);

        let graph = ActionGraph::new_with_cleanups(
            mocker.graph.into_graph(),
            FxHashSet::from_iter([task_b]),
        );

        assert_eq!(
            find_invariant_violations(&graph, true),
            vec![
                "RunTask(p:a) has multiple nodes",
                "optional edge RunTask(p:b) -> RunTask(p:a) (optional)",
                "non-task node links to a task SetupToolchain(tc-tier3:1.2.3) -> RunTask(p:a) (required)",
                "non-task cleanup or wait edge SetupToolchain(tc-tier2:1.2.3) -> SyncWorkspace (wait)",
                "SyncProject(p) doesn't reach SyncWorkspace",
                "RunTask(p:a) doesn't reach SyncWorkspace",
                "cleanup node RunTask(p:b) has no outgoing cleanup edge",
            ]
        );

        // Reachability is only required when the workspace is synced
        assert_eq!(find_invariant_violations(&graph, false).len(), 5);
    }
}
