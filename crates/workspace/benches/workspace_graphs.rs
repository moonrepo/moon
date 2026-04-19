use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use moon_bench_utils::{create_simple_workspace, handle_unwrap};
use moon_common::{is_ci, is_local};
use moon_test_utils2::WorkspaceMocker;
use moon_workspace::{WorkspaceBuilder, WorkspaceBuilderAsync};
use tokio::runtime::Runtime;

fn id(max: u16, label: &str) -> BenchmarkId {
    BenchmarkId::new(label, max)
}

fn should_run(max: u16) -> bool {
    is_local() || is_ci() && max <= 1000
}

fn do_limit(c: &mut Criterion, max: u16) {
    let mut group = c.benchmark_group("WorkspaceBuilder");
    let sandbox = create_simple_workspace(max);
    let mocker = WorkspaceMocker::new(sandbox.path()).load_default_configs();

    group.bench_function(id(max, "build_graphs_sync"), |b| {
        b.to_async(Runtime::new().unwrap()).iter(async || {
            let mut builder = WorkspaceBuilder::new(mocker.mock_workspace_builder_context())
                .await
                .unwrap();

            handle_unwrap(builder.load_projects().await);
            handle_unwrap(builder.load_tasks().await);
            handle_unwrap(builder.build().await);
        })
    });

    group.bench_function(id(max, "build_graphs_async"), |b| {
        b.to_async(Runtime::new().unwrap()).iter(async || {
            let mut builder = WorkspaceBuilderAsync::new(mocker.mock_workspace_builder_context())
                .await
                .unwrap();

            handle_unwrap(builder.load_graphs().await);
            handle_unwrap(builder.build().await);
        })
    });

    group.finish();
}

fn limit_100(c: &mut Criterion) {
    do_limit(c, 100);
}

fn limit_1000(c: &mut Criterion) {
    do_limit(c, 1000);
}

fn limit_5000(c: &mut Criterion) {
    // Too slow for CI!
    if should_run(5000) {
        do_limit(c, 5000);
    }
}

criterion_group!(benches, limit_100, limit_1000, limit_5000);
criterion_main!(benches);
