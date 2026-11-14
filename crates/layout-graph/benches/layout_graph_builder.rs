use criterion::{BatchSize, BenchmarkId, Criterion, criterion_group, criterion_main};
use moon_app_context::AppContext;
use moon_bench_utils::{create_layout_workspace, find_test_plugin};
use moon_common::Id;
use moon_layout_graph::LayoutGraphBuilder;
use moon_test_utils::{WorkspaceGraph, WorkspaceMocker};
use starbase_sandbox::Sandbox;
use std::path::Path;
use std::sync::Arc;
use tokio::runtime::Runtime;
use warpgate_api::{FileLocator, PluginLocator};

// Build and fully expand the workspace graph once, outside of the timed
// closures, as placing every project loads their tasks. The app context is
// shared too, as creating it loads the plugin registries.
fn create_graphs(
    runtime: &Runtime,
    sandbox: &Sandbox,
    wasm_path: &Path,
) -> (Arc<AppContext>, Arc<WorkspaceGraph>) {
    let mocker = WorkspaceMocker::new(sandbox.path())
        .load_default_configs()
        .update_toolchains_config(|config| {
            let plugin = config.plugins.entry(Id::raw("tc-tier2")).or_default();

            plugin.plugin = Some(PluginLocator::File(Box::new(FileLocator {
                file: format!("file://{}", wasm_path.display()),
                path: Some(wasm_path.to_path_buf()),
            })));
            plugin
                .config
                .insert("testLocateRoots".into(), serde_json::Value::Bool(true));
        });

    let graph = runtime.block_on(async {
        let graph = mocker.mock_workspace_graph().await;
        graph.get_tasks_with_internal().unwrap();
        graph
    });

    (Arc::new(mocker.mock_app_context()), Arc::new(graph))
}

// Place every project of the layout workspace, like `moon layout-graph`. The
// `tc-tier2` toolchain is loaded from the provided test plugin, which either
// locates one root per call (`tc_tier2`), or many (`tc_tier2_batch`). Skipped
// when the test plugin hasn't been built
fn do_place_all(c: &mut Criterion, group_name: &str, plugin_name: &str, max: u16) {
    let Some(wasm_path) = find_test_plugin(plugin_name) else {
        eprintln!(
            "Skipping the {group_name} benchmarks, as the {plugin_name} test plugin wasn't found (build it with `just build-wasm`)"
        );

        return;
    };

    // Plugin calls are memoized for the life of the process, which would make
    // every iteration after the first a cache hit, so it's disabled to measure
    // the calls, which a run pays for once.
    // SAFETY: Set before this benchmark's runtime and its threads are created,
    // and while no other threads are reading the environment.
    unsafe { std::env::set_var("WARPGATE_NO_FUNC_CACHE", "true") };

    let mut group = c.benchmark_group(group_name);
    let runtime = Runtime::new().unwrap();
    let sandbox = create_layout_workspace(max);
    let (app_context, graph) = create_graphs(&runtime, &sandbox, &wasm_path);

    if max >= 1000 {
        group.sample_size(10);
    }

    group.bench_function(BenchmarkId::new("place_all", max), |b| {
        b.to_async(&runtime).iter_batched(
            || (Arc::clone(&app_context), Arc::clone(&graph)),
            |(app_context, graph)| async move {
                let mut builder = LayoutGraphBuilder::new(app_context, graph);
                builder.place_all().await.unwrap();
                builder.build()
            },
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

fn single_10(c: &mut Criterion) {
    do_place_all(c, "LayoutGraphBuilder/single", "tc_tier2", 10);
}

fn single_100(c: &mut Criterion) {
    do_place_all(c, "LayoutGraphBuilder/single", "tc_tier2", 100);
}

fn single_1000(c: &mut Criterion) {
    do_place_all(c, "LayoutGraphBuilder/single", "tc_tier2", 1000);
}

fn batch_10(c: &mut Criterion) {
    do_place_all(c, "LayoutGraphBuilder/batch", "tc_tier2_batch", 10);
}

fn batch_100(c: &mut Criterion) {
    do_place_all(c, "LayoutGraphBuilder/batch", "tc_tier2_batch", 100);
}

fn batch_1000(c: &mut Criterion) {
    do_place_all(c, "LayoutGraphBuilder/batch", "tc_tier2_batch", 1000);
}

criterion_group!(
    benches,
    single_10,
    single_100,
    single_1000,
    batch_10,
    batch_100,
    batch_1000
);
criterion_main!(benches);
