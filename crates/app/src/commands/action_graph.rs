use crate::commands::graph::run_server;
use crate::session::MoonSession;
use clap::Args;
use moon_action_graph::{GraphToDot, GraphToJson, RunRequirements};
use moon_affected::DownstreamScope;
use moon_task::Target;
use starbase::AppResult;
use tracing::instrument;

#[derive(Args, Clone, Debug)]
pub struct ActionGraphArgs {
    #[arg(help = "Task targets to *only* graph")]
    targets: Option<Vec<Target>>,

    #[arg(long, help = "Include dependents of the focused target(s)")]
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
pub async fn action_graph(session: MoonSession, args: ActionGraphArgs) -> AppResult {
    let workspace_graph = session.get_workspace_graph().await?;
    let mut action_graph_builder = session.build_action_graph().await?;

    let requirements = RunRequirements {
        dependents: if args.dependents {
            DownstreamScope::Deep
        } else {
            DownstreamScope::None
        },
        ..Default::default()
    };

    // Focus a target and its dependencies/dependents
    if let Some(targets) = &args.targets {
        for target in targets {
            action_graph_builder
                .run_task_by_target(target, &requirements)
                .await?;
        }
    }
    // Show all targets and actions
    else {
        for task in workspace_graph.get_tasks()? {
            action_graph_builder.run_task(&task, &requirements).await?;
        }
    }

    let (_, action_graph) = action_graph_builder.build();

    if args.dot {
        session.console.out.write_line(action_graph.to_dot())?;

        return Ok(None);
    }

    if args.json {
        session
            .console
            .out
            .write_line(action_graph.to_json(true)?)?;

        return Ok(None);
    }

    run_server(
        "Action graph",
        action_graph.to_json(false)?,
        args.host,
        args.port,
    )
    .await?;

    Ok(None)
}
