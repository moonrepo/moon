use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use moon_bench_utils::{create_chained_workspace, create_simple_workspace, handle_unwrap};
use moon_common::{is_ci, is_local};
use moon_test_utils::WorkspaceMocker;
use moon_workspace::WorkspaceBuilder;
use starbase_sandbox::Sandbox;
use tokio::runtime::Runtime;

fn id(max: u16, label: &str) -> BenchmarkId {
    BenchmarkId::new(label, max)
}

fn should_run(max: u16) -> bool {
    is_local() || is_ci() && max <= 1000
}

fn do_limit(c: &mut Criterion, max: u16) {
    do_build(c, max, "build_graphs", create_simple_workspace(max));
}

// A single dependency chain (`p_n:build -> p_{n-1}:build -> ...`),
// which is the worst case for anything that traverses the graph
fn do_chained_limit(c: &mut Criterion, max: u16) {
    do_build(
        c,
        max,
        "build_graphs_chained",
        create_chained_workspace(max),
    );
}

fn do_build(c: &mut Criterion, max: u16, label: &str, sandbox: Sandbox) {
    let mut group = c.benchmark_group("WorkspaceBuilder");
    let mocker = WorkspaceMocker::new(sandbox.path()).load_default_configs();

    group.bench_function(id(max, label), |b| {
        b.to_async(Runtime::new().unwrap()).iter(async || {
            let mut builder = WorkspaceBuilder::new(mocker.mock_workspace_builder_context())
                .await
                .unwrap();

            handle_unwrap(builder.load_graphs().await);
            handle_unwrap(builder.build().await);
        })
    });

    group.finish();
}

// Build the graphs and then expand every project and task,
// as `moon ci`, `moon check`, and `moon query` do
fn do_expand_all(c: &mut Criterion, max: u16) {
    let sandbox = create_simple_workspace(max);
    let mut group = c.benchmark_group("WorkspaceBuilder");
    let mocker = WorkspaceMocker::new(sandbox.path()).load_default_configs();

    group.bench_function(id(max, "expand_all"), |b| {
        b.to_async(Runtime::new().unwrap()).iter(async || {
            let mut builder = WorkspaceBuilder::new(mocker.mock_workspace_builder_context())
                .await
                .unwrap();

            handle_unwrap(builder.load_graphs().await);

            let workspace_graph = builder.build().await.unwrap();

            handle_unwrap(workspace_graph.get_projects());
            handle_unwrap(workspace_graph.get_tasks_with_internal());
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

fn expand_all_1000(c: &mut Criterion) {
    do_expand_all(c, 1000);
}

fn chained_limit_1000(c: &mut Criterion) {
    do_chained_limit(c, 1000);
}

fn chained_limit_5000(c: &mut Criterion) {
    // Too slow for CI!
    if should_run(5000) {
        do_chained_limit(c, 5000);
    }
}

criterion_group!(
    benches,
    limit_100,
    limit_1000,
    limit_5000,
    chained_limit_1000,
    chained_limit_5000,
    expand_all_1000
);
criterion_main!(benches);
