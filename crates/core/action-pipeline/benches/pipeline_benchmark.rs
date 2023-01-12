use criterion::{black_box, criterion_group, criterion_main, Criterion};
use moon::{build_dep_graph, generate_project_graph, load_workspace_from};
use moon_action_pipeline::Pipeline;
use moon_dep_graph::DepGraph;
use moon_project_graph::ProjectGraph;
use moon_target::Target;
use moon_test_utils::{create_sandbox_with_config, get_cases_fixture_configs};
use moon_workspace::Workspace;

fn generate_dep_graph(workspace: &Workspace, project_graph: &ProjectGraph) -> DepGraph {
    let mut dep_graph = build_dep_graph(workspace, project_graph);

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

    dep_graph.build()
}

pub fn pipeline_benchmark(c: &mut Criterion) {
    let (workspace_config, toolchain_config, projects_config) = get_cases_fixture_configs();

    let sandbox = create_sandbox_with_config(
        "cases",
        Some(&workspace_config),
        Some(&toolchain_config),
        Some(&projects_config),
    );

    c.bench_function("pipeline", |b| {
        b.iter(|| async {
            let mut workspace = load_workspace_from(sandbox.path()).await.unwrap();
            let project_graph = generate_project_graph(&mut workspace).await.unwrap();
            let dep_graph = generate_dep_graph(&workspace, &project_graph);

            black_box(
                Pipeline::new(workspace, project_graph)
                    .run(dep_graph, None)
                    .await
                    .unwrap(),
            );
        })
    });
}

criterion_group!(pipeline, pipeline_benchmark);
criterion_main!(pipeline);
