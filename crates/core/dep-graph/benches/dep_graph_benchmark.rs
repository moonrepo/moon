use criterion::{criterion_group, criterion_main, Criterion};
use moon_config::NodeConfig;
use moon_dep_graph::{DepGraph, DepGraphBuilder};
use moon_node_platform::NodePlatform;
use moon_project_graph::{ProjectGraph, ProjectGraphBuilder};
use moon_system_platform::SystemPlatform;
use moon_task::Target;
use moon_test_utils::{create_sandbox_with_config, get_cases_fixture_configs};
use moon_workspace::Workspace;

pub fn setup_platforms(workspace: &mut Workspace) {
    workspace.register_platform(Box::new(SystemPlatform::default()));

    workspace.register_platform(Box::new(NodePlatform::new(
        &NodeConfig::default(),
        &workspace.root,
    )));
}

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

pub fn setup_dep_graph(workspace: &Workspace, project_graph: &ProjectGraph) -> DepGraph {
    let mut dep_graph = DepGraphBuilder::new(&workspace.platforms, project_graph);

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

pub fn load_benchmark(c: &mut Criterion) {
    c.bench_function("dep_graph_load", |b| {
        b.to_async(tokio::runtime::Runtime::new().unwrap())
            .iter(|| async {
                let (workspace_config, toolchain_config, projects_config) =
                    get_cases_fixture_configs();

                let sandbox = create_sandbox_with_config(
                    "cases",
                    Some(&workspace_config),
                    Some(&toolchain_config),
                    Some(&projects_config),
                );

                let mut workspace = Workspace::load_from(sandbox.path()).await.unwrap();
                let project_graph = generate_project_graph(&mut workspace).await;

                setup_dep_graph(&workspace, &project_graph);
            })
    });
}

pub fn load_with_platforms_benchmark(c: &mut Criterion) {
    c.bench_function("dep_graph_load_with_platforms", |b| {
        b.to_async(tokio::runtime::Runtime::new().unwrap())
            .iter(|| async {
                let (workspace_config, toolchain_config, projects_config) =
                    get_cases_fixture_configs();

                let sandbox = create_sandbox_with_config(
                    "cases",
                    Some(&workspace_config),
                    Some(&toolchain_config),
                    Some(&projects_config),
                );

                let mut workspace = Workspace::load_from(sandbox.path()).await.unwrap();
                let project_graph = generate_project_graph(&mut workspace).await;

                setup_platforms(&mut workspace);
                setup_dep_graph(&workspace, &project_graph);
            })
    });
}

criterion_group!(dep_graph, load_benchmark, load_with_platforms_benchmark);
criterion_main!(dep_graph);
