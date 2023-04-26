use criterion::{black_box, criterion_group, criterion_main, Criterion};
use moon::{build_dep_graph, generate_project_graph, load_workspace_from};
use moon_target::Target;
use moon_test_utils::{create_sandbox_with_config, get_cases_fixture_configs};

pub fn build_benchmark(c: &mut Criterion) {
    let (workspace_config, toolchain_config, tasks_config) = get_cases_fixture_configs();

    let sandbox = create_sandbox_with_config(
        "cases",
        Some(&workspace_config),
        Some(&toolchain_config),
        Some(&tasks_config),
    );

    c.bench_function("dep_graph_build", |b| {
        b.to_async(tokio::runtime::Runtime::new().unwrap())
            .iter(|| async {
                let mut workspace = load_workspace_from(sandbox.path()).await.unwrap();
                let project_graph = generate_project_graph(&mut workspace).await.unwrap();
                let mut dep_graph = build_dep_graph(&workspace, &project_graph);

                dep_graph
                    .run_target(Target::parse("base:base").unwrap(), None)
                    .unwrap();

                dep_graph
                    .run_target(Target::parse("depsA:dependencyOrder").unwrap(), None)
                    .unwrap();

                dep_graph
                    .run_target(Target::parse("outputs:withDeps").unwrap(), None)
                    .unwrap();

                dep_graph
                    .run_target(Target::parse("passthroughArgs:c").unwrap(), None)
                    .unwrap();

                dep_graph
                    .run_target(Target::parse("targetScopeB:self").unwrap(), None)
                    .unwrap();

                black_box(dep_graph.build());
            })
    });
}

criterion_group!(dep_graph, build_benchmark);
criterion_main!(dep_graph);
