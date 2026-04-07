use criterion::{Criterion, criterion_group, criterion_main};
use moon_affected::AffectedTracker;
use moon_bench_utils::create_simple_workspace;
use moon_common::path::WorkspaceRelativePathBuf;
use moon_test_utils2::WorkspaceMocker;
use rustc_hash::FxHashSet;
use starbase_sandbox::Sandbox;
use std::sync::Arc;
use tokio::runtime::Runtime;

fn create_changed_files(max: u16) -> FxHashSet<WorkspaceRelativePathBuf> {
    let mut set = FxHashSet::default();

    for i in (0..max).step_by(10) {
        set.insert(WorkspaceRelativePathBuf::from(format!("p{i}/file.txt")));
    }

    set
}

fn create_workspace_mocker(sandbox: &Sandbox) -> WorkspaceMocker {
    WorkspaceMocker::new(sandbox.path()).load_default_configs()
}

fn do_limit(c: &mut Criterion, max: u16) {
    let mut group = c.benchmark_group(format!("{max}"));
    let sandbox = create_simple_workspace(max);
    let files = create_changed_files(max);
    let mocker = create_workspace_mocker(&sandbox);

    group.bench_function("projects sync", |b| {
        b.to_async(Runtime::new().unwrap()).iter(async || {
            AffectedTracker::new(Arc::new(mocker.mock_workspace_graph().await), files.clone())
                .track_projects()
                .unwrap();
        })
    });

    group.bench_function("projects async", |b| {
        b.to_async(Runtime::new().unwrap()).iter(async || {
            AffectedTracker::new(Arc::new(mocker.mock_workspace_graph().await), files.clone())
                .track_projects_async()
                .await
                .unwrap();
        })
    });

    group.bench_function("tasks sync", |b| {
        b.to_async(Runtime::new().unwrap()).iter(async || {
            AffectedTracker::new(Arc::new(mocker.mock_workspace_graph().await), files.clone())
                .track_tasks()
                .unwrap();
        })
    });

    group.bench_function("tasks async", |b| {
        b.to_async(Runtime::new().unwrap()).iter(async || {
            AffectedTracker::new(Arc::new(mocker.mock_workspace_graph().await), files.clone())
                .track_tasks_async()
                .await
                .unwrap();
        })
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
    do_limit(c, 5000);
}

criterion_group!(benches, limit_10, limit_100, limit_1000, limit_5000);
criterion_main!(benches);
