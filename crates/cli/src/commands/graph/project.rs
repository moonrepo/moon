use crate::commands::graph::utils::{project_graph_repr, respond_to_request, setup_server};
use moon::{build_project_graph, load_workspace};

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
    let url = format!("http://{}", server.server_addr());
    let _ = open::that(&url);

    println!("Started server on {}", url);

    for req in server.incoming_requests() {
        respond_to_request(req, &mut tera, &graph_info, "Project graph".to_owned())?;
    }

    Ok(())
}
