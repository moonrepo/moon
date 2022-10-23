mod utils;

use criterion::{criterion_group, criterion_main, Criterion};
use moon_utils::test::get_fixtures_dir;
use moon_workspace::Workspace;

pub fn load_benchmark(c: &mut Criterion) {
    c.bench_function("dep_graph_load", |b| {
        b.iter(|| async {
            let workspace = Workspace::load_from(get_fixtures_dir("cases"))
                .await
                .unwrap();

            utils::setup_dep_graph(&workspace.projects);
        })
    });
}

pub fn load_with_platforms_benchmark(c: &mut Criterion) {
    c.bench_function("dep_graph_load_with_platforms", |b| {
        b.iter(|| async {
            let mut workspace = Workspace::load_from(get_fixtures_dir("cases"))
                .await
                .unwrap();

            utils::setup_platforms(&mut workspace);
            utils::setup_dep_graph(&workspace.projects);
        })
    });
}

criterion_group!(dep_graph, load_benchmark, load_with_platforms_benchmark);
criterion_main!(dep_graph);
