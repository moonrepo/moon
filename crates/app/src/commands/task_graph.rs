use crate::commands::graph::run_server;
use crate::session::MoonSession;
use clap::Args;
use moon_task::Target;
use moon_task_graph::{GraphToDot, GraphToJson};
use starbase::AppResult;
use std::sync::Arc;
use tracing::instrument;

#[derive(Args, Clone, Debug)]
pub struct TaskGraphArgs {
    #[arg(help = "Task target to *only* graph")]
    target: Option<Target>,

    #[arg(long, help = "Include direct dependents of the focused target")]
    dependents: bool,

    #[arg(
        long,
        help = "The host address",
        env = "MOON_HOST",
        default_value = "127.0.0.1"
    )]
    host: String,

    #[arg(
        long,
        help = "The port to bind to",
        env = "MOON_PORT",
        default_value = "0"
    )]
    port: u16,

    #[arg(long, help = "Print the graph in DOT format")]
    dot: bool,

    #[arg(long, help = "Print the graph in JSON format")]
    json: bool,
}

#[instrument(skip(session))]
pub async fn task_graph(session: MoonSession, args: TaskGraphArgs) -> AppResult {
    let mut task_graph = session.get_task_graph().await?;

    if let Some(target) = &args.target {
        task_graph = Arc::new(task_graph.focus_for(target, args.dependents)?);
    }

    // Force expand all tasks
    task_graph.get_all()?;

    if args.dot {
        session.console.out.write_line(task_graph.to_dot())?;

        return Ok(None);
    }

    if args.json {
        session.console.out.write_line(task_graph.to_json(true)?)?;

        return Ok(None);
    }

    run_server(
        "Task graph",
        task_graph.to_json(false)?,
        args.host,
        args.port,
    )
    .await?;

    Ok(None)
}
