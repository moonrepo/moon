use crate::commands::graph::{
    utils::{dep_graph_repr, respond_to_request, setup_server},
    LOG_TARGET,
};
use moon::{generate_project_graph, load_workspace};
use moon_logger::info;
use moon_runner::DepGraph;
use moon_task::Target;

pub async fn dep_graph(
    target_id: &Option<String>,
    dot: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut workspace = load_workspace().await?;
    let projects = generate_project_graph(&mut workspace).await?;
    let mut graph = DepGraph::default();

    // Preload all projects
    projects.load_all()?;

    // Focus a target and its dependencies/dependents
    if let Some(id) = target_id {
        let target = Target::parse(id)?;

        dep_builder.run_target(&target, None)?;
        dep_builder.run_dependents_for_target(&target)?;

    // Show all targets and actions
    } else {
        for project in project_graph.get_all()? {
            for task in project.tasks.values() {
                dep_builder.run_target(&task.target, None)?;
            }
        }
    }

    if dot {
        println!("{}", graph.to_dot());
    } else {
        let (server, mut tera) = setup_server().await?;
        let graph_info = dep_graph_repr(&graph).await;
        info!(
            target: LOG_TARGET,
            r#"Starting server on "{}""#,
            server.server_addr()
        );
        for req in server.incoming_requests() {
            respond_to_request(req, &mut tera, &graph_info, "Dependency".to_owned())?;
        }
    }

    Ok(())
}
