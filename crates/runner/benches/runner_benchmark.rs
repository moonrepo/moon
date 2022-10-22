use criterion::{criterion_group, criterion_main, Criterion};
use moon_action::ActionContext;
use moon_contract::Platformable;
use moon_platform_node::NodePlatform;
use moon_platform_system::SystemPlatform;
use moon_project_graph::ProjectGraph;
use moon_runner::{DepGraph, Runner};
use moon_task::Target;
use moon_utils::test::get_fixtures_dir;
use moon_workspace::Workspace;

fn setup_platforms(workspace: &mut Workspace) {
    workspace
        .projects
        .register_platform(Box::new(SystemPlatform::default()))
        .unwrap();

    workspace
        .projects
        .register_platform(Box::new(NodePlatform::default()))
        .unwrap();
}

fn setup_dep_graph(project_graph: &ProjectGraph) -> DepGraph {
    let mut dep_graph = DepGraph::default();

    dep_graph
        .run_target(Target::parse("base:base").unwrap(), project_graph, &None)
        .unwrap();

    dep_graph
        .run_target(
            Target::parse("depsA:dependencyOrder").unwrap(),
            project_graph,
            &None,
        )
        .unwrap();

    dep_graph
        .run_target(
            Target::parse("node:standard").unwrap(),
            project_graph,
            &None,
        )
        .unwrap();

    dep_graph
        .run_target(Target::parse("system:bash").unwrap(), project_graph, &None)
        .unwrap();

    dep_graph
        .run_target(
            Target::parse("targetScopeB:self").unwrap(),
            project_graph,
            &None,
        )
        .unwrap();

    dep_graph
}

pub fn load_dep_graph_benchmark(c: &mut Criterion) {
    c.bench_function("load_dep_graph", |b| {
        b.iter(|| async {
            let workspace = Workspace::create(get_fixtures_dir("cases")).await.unwrap();

            setup_dep_graph(&workspace.projects);
        })
    });
}

pub fn load_dep_graph_with_platforms_benchmark(c: &mut Criterion) {
    c.bench_function("load_dep_graph_with_platforms", |b| {
        b.iter(|| async {
            let mut workspace = Workspace::create(get_fixtures_dir("cases")).await.unwrap();

            setup_platforms(&mut workspace);

            setup_dep_graph(&workspace.projects);
        })
    });
}

criterion_group!(
    dep_graph,
    load_dep_graph_benchmark,
    load_dep_graph_with_platforms_benchmark
);

pub fn runner_benchmark(c: &mut Criterion) {
    c.bench_function("runner", |b| {
        b.iter(|| async {
            let workspace = Workspace::create(get_fixtures_dir("cases")).await.unwrap();

            let dep_graph = setup_dep_graph(&workspace.projects);

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
            let workspace = Workspace::create(get_fixtures_dir("cases")).await.unwrap();

            setup_platforms(&mut workspace);

            let dep_graph = setup_dep_graph(&workspace.projects);

            Runner::new(workspace)
                .run(dep_graph, Some(ActionContext::default()))
                .await
                .unwrap();
        })
    });
}

criterion_group!(runner, runner_benchmark, runner_with_platforms_benchmark);

criterion_main!(dep_graph, runner);
