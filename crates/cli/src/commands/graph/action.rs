use crate::commands::graph::utils::{action_graph_repr, respond_to_request, setup_server};
use clap::Args;
use miette::IntoDiagnostic;
use moon::{build_dep_graph, generate_project_graph};
use moon_target::TargetLocator;
use moon_workspace::Workspace;
use starbase::{system, SystemResult};

#[derive(Args, Clone, Debug)]
pub struct ActionGraphArgs {
    #[arg(help = "Target to *only* graph")]
    target: Option<TargetLocator>,

    #[arg(long, help = "Print the graph in DOT format")]
    dot: bool,

    #[arg(long, help = "Print the graph in JSON format")]
    json: bool,
}

pub async fn internal_action_graph(
    args: &ActionGraphArgs,
    workspace: &mut Workspace,
) -> SystemResult {
    let project_graph = generate_project_graph(workspace).await?;
    let mut action_graph_builder = build_dep_graph(&project_graph);

    // Focus a target and its dependencies/dependents
    if let Some(locator) = args.target.clone() {
        for target in action_graph_builder.run_targets_by_locator(&[locator], None)? {
            action_graph_builder.run_dependents_for_target(&target)?;
        }

        // Show all targets and actions
    } else {
        for project in project_graph.get_all_unexpanded() {
            for task in project.tasks.values() {
                action_graph_builder.run_target(&task.target, None)?;
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
        println!("{}", serde_json::to_string(&graph_info).into_diagnostic()?);

        return Ok(());
    }

    let (server, mut tera) = setup_server().await?;
    let url = format!("http://{}", server.server_addr());
    let _ = open::that(&url);

    println!("Started server on {url}");

    for req in server.incoming_requests() {
        respond_to_request(req, &mut tera, &graph_info, "Action graph".to_owned())?;
    }

    Ok(())
}

#[system]
pub async fn action_graph(args: ArgsRef<ActionGraphArgs>, workspace: ResourceMut<Workspace>) {
    internal_action_graph(args, workspace).await?;
}
