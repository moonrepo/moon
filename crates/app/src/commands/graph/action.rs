use crate::commands::graph::utils::{action_graph_repr, respond_to_request, setup_server};
use crate::session::CliSession;
use clap::Args;
use moon_action_graph::RunRequirements;
use moon_task::Target;
use starbase::AppResult;
use starbase_styles::color;
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
    let project_graph = session.get_project_graph().await?;
    let mut action_graph_builder = session.build_action_graph(&project_graph).await?;

    let mut requirements = RunRequirements {
        dependents: args.dependents,
        ..Default::default()
    };

    // Focus a target and its dependencies/dependents
    if let Some(targets) = &args.targets {
        for target in targets {
            action_graph_builder.run_task_by_target(target, &mut requirements)?;
        }

        // Show all targets and actions
    } else {
        for project in project_graph.get_all()? {
            for task in project.get_tasks()? {
                action_graph_builder.run_task(&project, task, &requirements)?;
            }
        }
    }

    let action_graph = action_graph_builder.build();

    if args.dot {
        println!("{}", action_graph.to_dot());

        return Ok(());
    }

    let graph_info = action_graph_repr(&action_graph).await;

    if args.json {
        println!("{}", json::format(&graph_info, false)?);

        return Ok(());
    }

    let (server, mut tera) = setup_server().await?;
    let url = format!("http://{}", server.server_addr());
    let _ = open::that(&url);

    println!("Started server on {}", color::url(url));

    for req in server.incoming_requests() {
        respond_to_request(req, &mut tera, &graph_info, "Action graph".to_owned())?;
    }

    Ok(())
}
