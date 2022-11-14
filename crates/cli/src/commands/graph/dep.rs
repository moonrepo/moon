use crate::commands::graph::utils::{dep_graph_repr, respond_to_request, setup_server};
use moon_runner::DepGraph;
use moon_task::Target;

pub async fn dep_graph(target_id: &Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    let (server, mut tera, workspace) = setup_server().await?;
    let projects = workspace.projects;
    let mut graph = DepGraph::default();

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

    let graph_info = dep_graph_repr(&graph).await;
    respond_to_request(server, &mut tera, &graph_info)?;

    println!("{}", graph.to_dot());

    Ok(())
}
