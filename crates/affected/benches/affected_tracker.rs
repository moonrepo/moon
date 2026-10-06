use criterion::{BatchSize, BenchmarkId, Criterion, criterion_group, criterion_main};
use moon_affected::{AffectedTracker, DownstreamScope, UpstreamScope};
use moon_bench_utils::{create_chained_workspace, create_simple_workspace};
use moon_common::is_local;
use moon_common::{is_ci, path::WorkspaceRelativePathBuf};
use moon_test_utils::{WorkspaceGraph, WorkspaceMocker};
use rustc_hash::FxHashSet;
use starbase_sandbox::Sandbox;
use std::sync::Arc;
use tokio::runtime::Runtime;

fn id(max: u16, label: &str) -> BenchmarkId {
    BenchmarkId::new(label, max)
}

fn should_run(max: u16) -> bool {
    is_local() || is_ci() && max <= 1000
}

fn create_changed_files(max: u16) -> FxHashSet<WorkspaceRelativePathBuf> {
    let mut set = FxHashSet::default();

    for i in (0..max).step_by(10) {
        set.insert(WorkspaceRelativePathBuf::from(format!("p{i}/file.txt")));
    }

    set
}

// Build and fully expand the workspace graph once, outside of the timed
// closures, so that only tracking is measured. Expanding tasks is lazy
// and would otherwise be paid for by the first iteration.
fn create_workspace_graph(runtime: &Runtime, sandbox: &Sandbox) -> Arc<WorkspaceGraph> {
    let mocker = WorkspaceMocker::new(sandbox.path()).load_default_configs();

    let graph = runtime.block_on(async {
        let graph = mocker.mock_workspace_graph().await;
        graph.get_tasks_with_internal().unwrap();
        graph
    });

    Arc::new(graph)
}

fn do_limit(c: &mut Criterion, max: u16) {
    let mut group = c.benchmark_group("AffectedTracker");
    let runtime = Runtime::new().unwrap();
    let sandbox = create_simple_workspace(max);
    let files = create_changed_files(max);
    let graph = create_workspace_graph(&runtime, &sandbox);

    if max >= 1000 {
        group.sample_size(10);
    }

    group.bench_function(id(max, "track_projects"), |b| {
        b.to_async(&runtime).iter_batched(
            || (Arc::clone(&graph), files.clone()),
            |(graph, files)| async move {
                AffectedTracker::new(graph, files)
                    .track_projects()
                    .await
                    .unwrap();
            },
            BatchSize::SmallInput,
        )
    });

    group.bench_function(id(max, "track_tasks"), |b| {
        b.to_async(&runtime).iter_batched(
            || (Arc::clone(&graph), files.clone()),
            |(graph, files)| async move {
                AffectedTracker::new(graph, files)
                    .track_tasks()
                    .await
                    .unwrap();
            },
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

// Every project (and its build task) sits in a single dependency chain, and
// deep scopes are enabled in both directions, so that the cost of walking
// relationships dominates over the cost of matching changed files
fn do_limit_chained(c: &mut Criterion, max: u16) {
    let mut group = c.benchmark_group("AffectedTracker/chained");
    let runtime = Runtime::new().unwrap();
    let sandbox = create_chained_workspace(max);
    let files = create_changed_files(max);
    let graph = create_workspace_graph(&runtime, &sandbox);

    if max >= 1000 {
        group.sample_size(10);
    }

    group.bench_function(id(max, "track_projects"), |b| {
        b.to_async(&runtime).iter_batched(
            || (Arc::clone(&graph), files.clone()),
            |(graph, files)| async move {
                let mut tracker = AffectedTracker::new(graph, files);
                tracker.set_project_scopes(UpstreamScope::Deep, DownstreamScope::Deep);
                tracker.track_projects().await.unwrap();
            },
            BatchSize::SmallInput,
        )
    });

    group.bench_function(id(max, "track_tasks"), |b| {
        b.to_async(&runtime).iter_batched(
            || (Arc::clone(&graph), files.clone()),
            |(graph, files)| async move {
                let mut tracker = AffectedTracker::new(graph, files);
                tracker.set_task_scopes(UpstreamScope::Deep, DownstreamScope::Deep);
                tracker.track_tasks().await.unwrap();
            },
            BatchSize::SmallInput,
        )
    });

    group.finish();
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
