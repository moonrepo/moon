use crate::{
    commands::graph::utils::{dep_graph_repr, respond_to_request, setup_server},
    helpers::AnyError,
};
use moon::{build_dep_graph, generate_project_graph, load_workspace};
use moon_target::Target;

pub async fn dep_graph(target_id: Option<String>, dot: bool, json: bool) -> Result<(), AnyError> {
    let mut workspace = load_workspace().await?;
    let project_graph = generate_project_graph(&mut workspace).await?;
    let mut dep_builder = build_dep_graph(&workspace, &project_graph);

    // Focus a target and its dependencies/dependents
    if let Some(id) = &target_id {
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

    let dep_graph = dep_builder.build();

    if dot {
        println!("{}", dep_graph.to_dot());

        return Ok(());
    }

    let graph_info = dep_graph_repr(&dep_graph).await;

    if json {
        println!("{}", serde_json::to_string(&graph_info)?);

        return Ok(());
    }

    let (server, mut tera) = setup_server().await?;
    let url = format!("http://{}", server.server_addr());
    let _ = open::that(&url);

    println!("Started server on {url}");

    for req in server.incoming_requests() {
        respond_to_request(req, &mut tera, &graph_info, "Dependency graph".to_owned())?;
    }

    Ok(())
}
