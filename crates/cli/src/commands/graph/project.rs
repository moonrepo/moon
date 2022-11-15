use moon_logger::info;

use crate::{
    commands::graph::{
        utils::{respond_to_request, setup_server, workspace_graph_repr},
        LOG_TARGET,
    },
    helpers::load_workspace,
};

pub async fn project_graph(
    project_id: &Option<String>,
    dot: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let workspace = load_workspace().await?;

    if let Some(id) = project_id {
        project_build.load(id)?;
    } else {
        project_build.load_all()?;
    }

    if dot {
        println!("{}", workspace.projects.to_dot());
    } else {
        let (server, mut tera) = setup_server().await?;
        let graph_info = workspace_graph_repr(&workspace).await;
        info!(
            target: LOG_TARGET,
            r#"Starting server on "{}""#,
            server.server_addr()
        );
        for req in server.incoming_requests() {
            respond_to_request(req, &mut tera, &graph_info)?;
        }
    }
    Ok(())
}
