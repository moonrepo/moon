use crate::commands::graph::utils::{action_graph_repr, run_server};
use crate::session::CliSession;
use clap::Args;
use moon_action_graph::RunRequirements;
use moon_task::Target;
use starbase::AppResult;
use starbase_utils::json;
use tracing::instrument;

#[derive(Args, Clone, Debug)]
pub struct ActionGraphArgs {
    #[arg(help = "Targets to *only* graph")]
    targets: Option<Vec<Target>>,

    #[arg(long, help = "Include dependents of the focused target(s)")]
    dependents: bool,

    #[arg(long, help = "Print the graph in DOT format")]
    dot: bool,

    #[arg(long, help = "Print the graph in JSON format")]
    json: bool,
}

#[instrument]
pub async fn action_graph(session: CliSession, args: ActionGraphArgs) -> AppResult {
    let workspace_graph = session.get_workspace_graph().await?;
    let mut action_graph_builder = session.build_action_graph(&workspace_graph).await?;

    let requirements = RunRequirements {
        dependents: args.dependents,
        ..Default::default()
    };

    // Focus a target and its dependencies/dependents
    if let Some(targets) = &args.targets {
        for target in targets {
            action_graph_builder.run_task_by_target(target, &requirements)?;
        }
    }
    // Show all targets and actions
    else {
        for project in workspace_graph.get_projects()? {
            for task in workspace_graph.get_tasks_from_project(&project.id)? {
                action_graph_builder.run_task(&project, &task, &requirements)?;
            }
        }
    }

    let action_graph = action_graph_builder.build();

    if args.dot {
        println!("{}", action_graph.to_dot());

        return Ok(None);
    }

    let graph_info = action_graph_repr(&action_graph).await;

    if args.json {
        println!("{}", json::format(&graph_info, true)?);

        return Ok(None);
    }

    run_server("Action graph", graph_info).await?;

    Ok(None)
}
