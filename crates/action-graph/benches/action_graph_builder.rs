use criterion::{BatchSize, BenchmarkId, Criterion, criterion_group, criterion_main};
use moon_action_graph::{ActionGraphBuilder, ActionGraphBuilderOptions, RunRequirements};
use moon_app_context::AppContext;
use moon_bench_utils::{
    create_chained_workspace, create_serial_workspace, create_simple_workspace,
    create_toolchain_workspace,
};
use moon_common::{Id, is_ci, is_local};
use moon_config::UnresolvedVersionSpec;
use moon_task::TargetLocator;
use moon_test_utils::{WorkspaceGraph, WorkspaceMocker};
use starbase_sandbox::Sandbox;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::runtime::{Builder, Runtime};
use warpgate_api::{FileLocator, PluginLocator};

// The previous builder recursed (through boxed futures) once per task dependency
// and project dependency, which overflowed the stack for the 1000 project chain,
// so every build was spawned onto a runtime worker with a larger stack, instead
// of being polled on the main thread by `block_on`. The current builder doesn't
// need it, but builds are still spawned the same way, so that results remain
// comparable with the ones in docs/todo/ACTION_GRAPH_BUILDER.md. Spawning
// measures 9% to 32% slower (the most for the smallest cases)
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
    configure: impl FnOnce(WorkspaceMocker) -> WorkspaceMocker,
) -> (Arc<AppContext>, Arc<WorkspaceGraph>, Vec<TargetLocator>) {
    let mocker = configure(WorkspaceMocker::new(sandbox.path()).load_default_configs());

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
    bench_run_tasks_with(c, group_name, max, sandbox, |mocker| mocker);
}

fn bench_run_tasks_with(
    c: &mut Criterion,
    group_name: &str,
    max: u16,
    sandbox: Sandbox,
    configure: impl FnOnce(WorkspaceMocker) -> WorkspaceMocker,
) {
    let mut group = c.benchmark_group(group_name);
    let runtime = create_runtime();
    let (app_context, graph, locators) = create_graphs(&runtime, &sandbox, configure);

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

// Find a test plugin built with `just build-wasm`, which builds into
// `CARGO_TARGET_DIR` when it's set, otherwise into `wasm/target`. Benchmarks are
// release builds, which don't resolve the test plugins' locators, so it's
// located here
fn find_test_plugin(name: &str) -> Option<PathBuf> {
    let file = format!("{name}.wasm");

    ["CARGO_TARGET_DIR", "WARPGATE_PLUGINS_DIR"]
        .into_iter()
        .filter_map(std::env::var_os)
        .map(PathBuf::from)
        .flat_map(|dir| [dir.clone(), dir.join("target")])
        .flat_map(|dir| {
            ["release", "debug"].map(|profile| dir.join("wasm32-wasip1").join(profile).join(&file))
        })
        .find(|path| path.exists())
}

// Like the simple workspace, but every task runs in a tier 3 toolchain. The
// test plugin locates the dependencies root at the workspace root, without
// members, so no project is in it, and every task falls back to setting up the
// toolchain. This measures resolving that (the toolchain's requirements and
// supported tiers, and locating the root), but not install or environment
// actions, nor matching workspace members. Skipped when the test plugin hasn't
// been built
fn do_limit_toolchain(c: &mut Criterion, max: u16) {
    let Some(wasm_path) = find_test_plugin("tc_tier3") else {
        eprintln!(
            "Skipping the toolchain benchmarks, as the tc_tier3 test plugin wasn't found (build it with `just build-wasm`)"
        );

        return;
    };

    bench_run_tasks_with(
        c,
        "ActionGraphBuilder/toolchain",
        max,
        create_toolchain_workspace(max),
        |mocker| {
            mocker.update_toolchains_config(|config| {
                let plugin = config.plugins.entry(Id::raw("tc-tier3")).or_default();

                plugin.plugin = Some(PluginLocator::File(Box::new(FileLocator {
                    file: format!("file://{}", wasm_path.display()),
                    path: Some(wasm_path.clone()),
                })));
                plugin.version = Some(UnresolvedVersionSpec::parse("1.2.3").unwrap());
            })
        },
    );
}

// Like the chained workspace, but every `build` task runs its dependencies
// serially, so the chain of `build` tasks below each one is ordered after
// its `test` task
fn do_limit_serial(c: &mut Criterion, max: u16) {
    bench_run_tasks(
        c,
        "ActionGraphBuilder/serial",
        max,
        create_serial_workspace(max),
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

fn toolchain_10(c: &mut Criterion) {
    do_limit_toolchain(c, 10);
}

fn toolchain_100(c: &mut Criterion) {
    do_limit_toolchain(c, 100);
}

fn toolchain_1000(c: &mut Criterion) {
    do_limit_toolchain(c, 1000);
}

fn serial_10(c: &mut Criterion) {
    do_limit_serial(c, 10);
}

fn serial_100(c: &mut Criterion) {
    do_limit_serial(c, 100);
}

fn serial_1000(c: &mut Criterion) {
    do_limit_serial(c, 1000);
}

criterion_group!(
    benches,
    limit_10,
    limit_100,
    limit_1000,
    limit_5000,
    chained_10,
    chained_100,
    chained_1000,
    toolchain_10,
    toolchain_100,
    toolchain_1000,
    serial_10,
    serial_100,
    serial_1000
);
criterion_main!(benches);
