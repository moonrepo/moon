use criterion::{black_box, criterion_group, criterion_main, Criterion};
use moon_cache::CacheEngine;
use moon_config::{GlobalProjectConfig, WorkspaceConfig, WorkspaceProjects};
use moon_project_graph::ProjectGraph;
use moon_utils::test::get_fixtures_dir;
use std::collections::HashMap;

pub fn load_benchmark(c: &mut Criterion) {
    let workspace_root = get_fixtures_dir("cases");
    let workspace_config = WorkspaceConfig {
        projects: WorkspaceProjects::Sources(HashMap::from([(
            "base".to_owned(),
            "base".to_owned(),
        )])),
        ..WorkspaceConfig::default()
    };

    let runtime = tokio::runtime::Runtime::new().unwrap();

    runtime.block_on(async {
        let cache = CacheEngine::load(&workspace_root).await.unwrap();

        let graph = ProjectGraph::generate(
            &workspace_root,
            &workspace_config,
            GlobalProjectConfig::default(),
            &cache,
        )
        .await
        .unwrap();

        c.bench_function("project_graph_load", |b| {
            b.iter(|| {
                // This clones a new project struct every time
                black_box(graph.load("base").unwrap());
            })
        });
    });
}

pub fn load_all_benchmark(c: &mut Criterion) {
    let workspace_root = get_fixtures_dir("cases");
    let workspace_config = WorkspaceConfig::default();

    c.bench_function("project_graph_load_all", |b| {
        b.to_async(tokio::runtime::Runtime::new().unwrap())
            .iter(|| async {
                let cache = CacheEngine::load(&workspace_root).await.unwrap();
                let graph = ProjectGraph::generate(
                    &workspace_root,
                    &workspace_config,
                    GlobalProjectConfig::default(),
                    &cache,
                )
                .await
                .unwrap();

                // This does NOT clone but inserts all projects into the graph
                graph.load_all().unwrap();
            })
    });
}

criterion_group!(project_graph, load_benchmark, load_all_benchmark);
criterion_main!(project_graph);
