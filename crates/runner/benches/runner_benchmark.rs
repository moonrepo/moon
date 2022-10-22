mod utils;

use criterion::{criterion_group, criterion_main, Criterion};
use moon_action::ActionContext;
use moon_runner::Runner;
use moon_utils::test::get_fixtures_dir;
use moon_workspace::Workspace;

pub fn runner_benchmark(c: &mut Criterion) {
    c.bench_function("runner", |b| {
        b.iter(|| async {
            let workspace = Workspace::create(get_fixtures_dir("cases")).await.unwrap();

            let dep_graph = utils::setup_dep_graph(&workspace.projects);

            Runner::new(workspace)
                .run(dep_graph, Some(ActionContext::default()))
                .await
                .unwrap();
        })
    });
}

pub fn runner_with_platforms_benchmark(c: &mut Criterion) {
    c.bench_function("runner_with_platforms", |b| {
        b.iter(|| async {
            let mut workspace = Workspace::create(get_fixtures_dir("cases")).await.unwrap();

            utils::setup_platforms(&mut workspace);

            let dep_graph = utils::setup_dep_graph(&workspace.projects);

            Runner::new(workspace)
                .run(dep_graph, Some(ActionContext::default()))
                .await
                .unwrap();
        })
    });
}

criterion_group!(runner, runner_benchmark, runner_with_platforms_benchmark);
criterion_main!(runner);
