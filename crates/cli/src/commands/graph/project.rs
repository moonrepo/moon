use crate::{
    commands::graph::utils::{respond_to_request, setup_server, workspace_graph_repr},
    helpers::load_workspace,
};

pub async fn project_graph(
    project_id: &Option<String>,
    dot: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let workspace = load_workspace().await?;
    let (server, mut tera) = setup_server().await?;

    if let Some(id) = project_id {
        project_build.load(id)?;
    } else {
        project_build.load_all()?;
    }

    let graph_info = workspace_graph_repr(&workspace).await;

    if dot {
        println!("{}", workspace.projects.to_dot());
    } else {
        respond_to_request(server, &mut tera, &graph_info)?;
    }
    Ok(())
}
