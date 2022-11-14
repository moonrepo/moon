use crate::commands::graph::utils::{respond_to_request, setup_server, workspace_graph_repr};

pub async fn project_graph(project_id: &Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    let (server, mut tera, workspace) = setup_server().await?;
    let graph_info = workspace_graph_repr(&workspace).await;

    respond_to_request(server, &mut tera, &graph_info)?;

    if let Some(id) = project_id {
        project_build.load(id)?;
    } else {
        project_build.load_all()?;
    }

    let project_graph = project_build.build();

    println!("{}", project_graph.to_dot());

    Ok(())
}
