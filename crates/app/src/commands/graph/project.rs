use super::utils::{project_graph_repr, run_server};
use crate::session::MoonSession;
use clap::Args;
use moon_common::Id;
use moon_project_graph::{GraphToDot, GraphToJson};
use starbase::AppResult;
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
pub async fn project_graph(session: MoonSession, args: ProjectGraphArgs) -> AppResult {
    let mut project_graph = session.get_project_graph().await?;

    if let Some(id) = &args.id {
        project_graph = Arc::new(project_graph.focus_for(id, args.dependents)?);
    }

    // Force expand all projects
    project_graph.get_all()?;

    if args.dot {
        session.console.out.write_line(project_graph.to_dot())?;

        return Ok(None);
    }

    if args.json {
        session.console.out.write_line(project_graph.to_json()?)?;

        return Ok(None);
    }

    run_server("Project graph", project_graph_repr(&project_graph).await).await?;

    Ok(None)
}
