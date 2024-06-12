use super::utils::{project_graph_repr, respond_to_request, setup_server};
use crate::session::CliSession;
use clap::Args;
use moon_common::Id;
use starbase::AppResult;
use starbase_styles::color;
use std::sync::Arc;
use tracing::instrument;

#[derive(Args, Clone, Debug)]
pub struct ProjectGraphArgs {
    #[arg(help = "ID of project to *only* graph")]
    id: Option<Id>,

    #[arg(long, help = "Include direct dependents of the focused project")]
    dependents: bool,

    #[arg(long, help = "Print the graph in DOT format")]
    dot: bool,

    #[arg(long, help = "Print the graph in JSON format")]
    json: bool,
}

#[instrument(skip_all)]
pub async fn project_graph(session: CliSession, args: ProjectGraphArgs) -> AppResult {
    let mut project_graph = session.get_project_graph().await?;

    if let Some(id) = &args.id {
        project_graph = Arc::new(project_graph.into_focused(id, args.dependents)?);
    }

    // Force expand all projects
    project_graph.get_all()?;

    if args.dot {
        println!("{}", project_graph.to_dot());

        return Ok(());
    }

    if args.json {
        println!("{}", project_graph.to_json()?);

        return Ok(());
    }

    let graph_info = project_graph_repr(&project_graph).await;
    let (server, mut tera) = setup_server().await?;
    let url = format!("http://{}", server.server_addr());
    let _ = open::that(&url);

    println!("Started server on {}", color::url(url));

    for req in server.incoming_requests() {
        respond_to_request(req, &mut tera, &graph_info, "Project graph".to_owned())?;
    }

    Ok(())
}
