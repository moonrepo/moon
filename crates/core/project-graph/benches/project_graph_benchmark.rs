use criterion::{criterion_group, criterion_main, Criterion};
use moon::{generate_project_graph, load_workspace_from};
use moon_test_utils::{create_sandbox_with_config, get_cases_fixture_configs};

pub fn get_benchmark(c: &mut Criterion) {
    let (workspace_config, toolchain_config, projects_config) = get_cases_fixture_configs();

    let sandbox = create_sandbox_with_config(
        "cases",
        Some(&workspace_config),
        Some(&toolchain_config),
        Some(&projects_config),
    );

    c.bench_function("project_graph_get", |b| {
        b.to_async(tokio::runtime::Runtime::new().unwrap())
            .iter(|| async {
                let mut workspace = load_workspace_from(sandbox.path()).await.unwrap();
                let graph = generate_project_graph(&mut workspace).await.unwrap();

                for _ in 0..1000 {
                    graph.get("base").unwrap();
                }
            })
    });
}

pub fn get_all_benchmark(c: &mut Criterion) {
    let (workspace_config, toolchain_config, projects_config) = get_cases_fixture_configs();

    let sandbox = create_sandbox_with_config(
        "cases",
        Some(&workspace_config),
        Some(&toolchain_config),
        Some(&projects_config),
    );

    c.bench_function("project_graph_get_all", |b| {
        b.to_async(tokio::runtime::Runtime::new().unwrap())
            .iter(|| async {
                let mut workspace = load_workspace_from(sandbox.path()).await.unwrap();
                let graph = generate_project_graph(&mut workspace).await.unwrap();

                for _ in 0..1000 {
                    graph.get_all().unwrap();
                }
            })
    });
}

criterion_group!(project_graph, get_benchmark, get_all_benchmark);
criterion_main!(project_graph);
