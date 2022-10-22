use moon_contract::Platformable;
use moon_platform_node::NodePlatform;
use moon_platform_system::SystemPlatform;
use moon_project_graph::ProjectGraph;
use moon_runner::DepGraph;
use moon_task::Target;
use moon_workspace::Workspace;

pub fn setup_platforms(workspace: &mut Workspace) {
    workspace
        .projects
        .register_platform(Box::new(SystemPlatform::default()))
        .unwrap();

    workspace
        .projects
        .register_platform(Box::new(NodePlatform::default()))
        .unwrap();
}

pub fn setup_dep_graph(project_graph: &ProjectGraph) -> DepGraph {
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
