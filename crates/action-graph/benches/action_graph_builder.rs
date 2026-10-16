use criterion::{BatchSize, BenchmarkId, Criterion, criterion_group, criterion_main};
use moon_action_graph::ActionGraphBuilder as NewActionGraphBuilder;
use moon_action_graph::legacy::ActionGraphBuilder;
use moon_action_graph::{ActionGraphBuilderOptions, RunRequirements};
use moon_app_context::AppContext;
use moon_bench_utils::{create_chained_workspace, create_simple_workspace};
use moon_common::{is_ci, is_local};
use moon_task::TargetLocator;
use moon_test_utils::{WorkspaceGraph, WorkspaceMocker};
use starbase_sandbox::Sandbox;
use std::sync::Arc;
use tokio::runtime::{Builder, Runtime};

// The current builder recurses (through boxed futures) once per task dependency
// and project dependency, which overflows the 8MB stack of the main thread for
// the 1000 project chain (900 still fits). So every build is spawned onto a
// runtime worker with a larger stack, instead of being polled on the main thread
// by `block_on`. That measures 9% to 32% slower (the most for the smallest
// cases), but measures every builder, and every workspace size, the same way
const WORKER_STACK_SIZE: usize = 256 * 1024 * 1024;

fn create_runtime() -> Runtime {
    Builder::new_multi_thread()
        .thread_stack_size(WORKER_STACK_SIZE)
        .enable_all()
        .build()
        .unwrap()
}

fn id(max: u16, label: &str) -> BenchmarkId {
    BenchmarkId::new(label, max)
}

fn should_run(max: u16) -> bool {
    is_local() || is_ci() && max <= 1000
}

// Build and fully expand the workspace graph once, outside of the timed
// closures, so that only building the action graph is measured. Expanding
// tasks is lazy and would otherwise be paid for by the first iteration.
// The app context is shared too, as creating it loads the plugin registries.
fn create_graphs(
    runtime: &Runtime,
    sandbox: &Sandbox,
) -> (Arc<AppContext>, Arc<WorkspaceGraph>, Vec<TargetLocator>) {
    let mocker = WorkspaceMocker::new(sandbox.path()).load_default_configs();

    let graph = runtime.block_on(async {
        let graph = mocker.mock_workspace_graph().await;
        graph.get_tasks_with_internal().unwrap();
        graph
    });

    let mut locators = graph
        .get_tasks()
        .unwrap()
        .into_iter()
        .map(|task| TargetLocator::Qualified(task.target.clone()))
        .collect::<Vec<_>>();

    locators.sort_by(|a, b| a.as_str().cmp(b.as_str()));

    (
        Arc::new(mocker.mock_app_context()),
        Arc::new(graph),
        locators,
    )
}

fn bench_run_tasks(c: &mut Criterion, group_name: &str, max: u16, sandbox: Sandbox) {
    let mut group = c.benchmark_group(group_name);
    let runtime = create_runtime();
    let (app_context, graph, locators) = create_graphs(&runtime, &sandbox);

    if max >= 1000 {
        group.sample_size(10);
    }

    // Like `moon run` with every task as a target
    group.bench_function(id(max, "run_tasks"), |b| {
        b.to_async(&runtime).iter_batched(
            || {
                (
                    Arc::clone(&app_context),
                    Arc::clone(&graph),
                    locators.clone(),
                )
            },
            |(app_context, graph, locators)| async move {
                tokio::spawn(async move {
                    let mut builder = ActionGraphBuilder::new(
                        app_context,
                        graph,
                        ActionGraphBuilderOptions::default(),
                    )
                    .unwrap();

                    builder
                        .run_tasks(
                            locators,
                            RunRequirements {
                                ci: false,
                                ..Default::default()
                            },
                        )
                        .await
                        .unwrap();

                    builder.build()
                })
                .await
                .unwrap()
            },
            BatchSize::SmallInput,
        )
    });

    // The same, with the new builder (see docs/todo/ACTION_GRAPH_BUILDER.md)
    group.bench_function(id(max, "new_run_tasks"), |b| {
        b.to_async(&runtime).iter_batched(
            || {
                (
                    Arc::clone(&app_context),
                    Arc::clone(&graph),
                    locators.clone(),
                )
            },
            |(app_context, graph, locators)| async move {
                tokio::spawn(async move {
                    let mut builder = NewActionGraphBuilder::new(
                        app_context,
                        graph,
                        ActionGraphBuilderOptions::default(),
                    );

                    builder.set_requirements(RunRequirements {
                        ci: false,
                        ..Default::default()
                    });
                    builder.run_tasks(locators);
                    builder.build().await.unwrap()
                })
                .await
                .unwrap()
            },
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

// Every project has 4 tasks (3 of which are a chain), and
// no project dependencies, so the graph is wide and shallow
fn do_limit(c: &mut Criterion, max: u16) {
    bench_run_tasks(c, "ActionGraphBuilder", max, create_simple_workspace(max));
}

// Every project (and its build task) sits in a single dependency chain,
// so the graph is as deep as there are projects, and every task and
// project sync has a deep upstream
fn do_limit_chained(c: &mut Criterion, max: u16) {
    bench_run_tasks(
        c,
        "ActionGraphBuilder/chained",
        max,
        create_chained_workspace(max),
    );
}

fn limit_10(c: &mut Criterion) {
    do_limit(c, 10);
}

fn limit_100(c: &mut Criterion) {
    do_limit(c, 100);
}

fn limit_1000(c: &mut Criterion) {
    do_limit(c, 1000);
}

fn limit_5000(c: &mut Criterion) {
    if should_run(5000) {
        do_limit(c, 5000);
    }
}

fn chained_10(c: &mut Criterion) {
    do_limit_chained(c, 10);
}

fn chained_100(c: &mut Criterion) {
    do_limit_chained(c, 100);
}

fn chained_1000(c: &mut Criterion) {
    do_limit_chained(c, 1000);
}

criterion_group!(
    benches,
    limit_10,
    limit_100,
    limit_1000,
    limit_5000,
    chained_10,
    chained_100,
    chained_1000
);
criterion_main!(benches);
