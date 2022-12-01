use criterion::{black_box, criterion_group, criterion_main, Criterion};
use moon_project_graph::{ProjectGraph, ProjectGraphBuilder};
use moon_test_utils::{create_sandbox_with_config, get_cases_fixture_configs};
use moon_workspace::Workspace;

async fn generate_project_graph(workspace: &mut Workspace) -> ProjectGraph {
    let mut builder = ProjectGraphBuilder {
        cache: &workspace.cache,
        config: &workspace.projects_config,
        platforms: &mut workspace.platforms,
        workspace_config: &workspace.config,
        workspace_root: &workspace.root,
    };

    builder.build().await.unwrap()
}

pub fn load_benchmark(c: &mut Criterion) {
    let (workspace_config, toolchain_config, projects_config) = get_cases_fixture_configs();

    let sandbox = create_sandbox_with_config(
        "cases",
        Some(&workspace_config),
        Some(&toolchain_config),
        Some(&projects_config),
    );

    c.bench_function("project_graph_load", |b| {
        b.to_async(tokio::runtime::Runtime::new().unwrap())
            .iter(|| async {
                let mut workspace = Workspace::load_from(sandbox.path()).await.unwrap();
                let graph = generate_project_graph(&mut workspace).await;

                black_box(graph.get("base").unwrap());
            })
    });
}

pub fn load_all_benchmark(c: &mut Criterion) {
    let (workspace_config, toolchain_config, projects_config) = get_cases_fixture_configs();

    let sandbox = create_sandbox_with_config(
        "cases",
        Some(&workspace_config),
        Some(&toolchain_config),
        Some(&projects_config),
    );

    c.bench_function("project_graph_load_all", |b| {
        b.to_async(tokio::runtime::Runtime::new().unwrap())
            .iter(|| async {
                let mut workspace = Workspace::load_from(sandbox.path()).await.unwrap();
                let graph = generate_project_graph(&mut workspace).await;

                black_box(graph.get_all().unwrap());
            })
    });
}

criterion_group!(project_graph, load_benchmark, load_all_benchmark);
criterion_main!(project_graph);
