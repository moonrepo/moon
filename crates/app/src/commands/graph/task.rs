use super::utils::{respond_to_request, setup_server, task_graph_repr};
use crate::session::CliSession;
use clap::Args;
use moon_task::Target;
use starbase::AppResult;
use starbase_styles::color;
use std::sync::Arc;
use tracing::instrument;

#[derive(Args, Clone, Debug)]
pub struct TaskGraphArgs {
    #[arg(help = "Target of task to *only* graph")]
    target: Option<Target>,

    #[arg(long, help = "Include direct dependents of the focused task")]
    dependents: bool,

    #[arg(long, help = "Print the graph in DOT format")]
    dot: bool,

    #[arg(long, help = "Print the graph in JSON format")]
    json: bool,
}

#[instrument(skip_all)]
pub async fn task_graph(session: CliSession, args: TaskGraphArgs) -> AppResult {
    let mut task_graph = session.get_task_graph().await?;

    if let Some(target) = &args.target {
        task_graph = Arc::new(task_graph.into_focused(target, args.dependents)?);
    }

    if args.dot {
        println!("{}", task_graph.to_dot());

        return Ok(());
    }

    if args.json {
        println!("{}", task_graph.to_json()?);

        return Ok(());
    }

    let graph_info = task_graph_repr(&task_graph).await;
    let (server, mut tera) = setup_server().await?;
    let url = format!("http://{}", server.server_addr());
    let _ = open::that(&url);

    println!("Started server on {}", color::url(url));

    for req in server.incoming_requests() {
        respond_to_request(req, &mut tera, &graph_info, "Task graph".to_owned())?;
    }

    Ok(())
}
