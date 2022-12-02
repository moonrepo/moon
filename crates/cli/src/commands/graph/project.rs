use crate::commands::graph::{
    utils::{project_graph_repr, respond_to_request, setup_server},
    LOG_TARGET,
};
use moon::{build_project_graph, load_workspace};
use moon_logger::info;

pub async fn project_graph(
    project_id: &Option<String>,
    dot: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut workspace = load_workspace().await?;
    let mut project_build = build_project_graph(&mut workspace)?;

    if let Some(id) = project_id {
        project_build.load(id)?;
    } else {
        project_build.load_all()?;
    }

    let project_graph = project_build.build();

    if dot {
        println!("{}", project_graph.to_dot());

        return Ok(());
    }

    let (server, mut tera) = setup_server().await?;
    let graph_info = project_graph_repr(&project_graph).await;

    info!(
        target: LOG_TARGET,
        r#"Starting server on "{}""#,
        server.server_addr()
    );

    for req in server.incoming_requests() {
        respond_to_request(req, &mut tera, &graph_info, "Project".to_owned())?;
    }

    Ok(())
}
