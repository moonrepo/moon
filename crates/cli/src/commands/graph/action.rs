use crate::commands::graph::utils::{action_graph_repr, respond_to_request, setup_server};
use clap::Args;
use miette::IntoDiagnostic;
use moon::{build_action_graph, generate_project_graph};
use moon_action_graph::RunRequirements;
use moon_target::TargetLocator;
use moon_workspace::Workspace;
use starbase::{system, SystemResult};
use starbase_styles::color;

#[derive(Args, Clone, Debug)]
pub struct ActionGraphArgs {
    #[arg(help = "Targets to *only* graph")]
    targets: Option<Vec<TargetLocator>>,

    #[arg(long, help = "Include dependents of the focused target(s)")]
    dependents: bool,

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
    let mut action_graph_builder = build_action_graph(&project_graph)?;

    let requirements = RunRequirements {
        dependents: args.dependents,
        ..Default::default()
    };

    // Focus a target and its dependencies/dependents
    if let Some(locators) = &args.targets {
        for locator in locators {
            action_graph_builder.run_task_by_target_locator(locator, &requirements)?;
        }

        // Show all targets and actions
    } else {
        for project in project_graph.get_all()? {
            for task in project.tasks.values() {
                action_graph_builder.run_task(&project, task, &requirements)?;
            }
        }
    }

    let action_graph = action_graph_builder.build()?;

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

    println!("Started server on {}", color::url(url));

    for req in server.incoming_requests() {
        respond_to_request(req, &mut tera, &graph_info, "Action graph".to_owned())?;
    }

    Ok(())
}

#[system]
pub async fn action_graph(args: ArgsRef<ActionGraphArgs>, workspace: ResourceMut<Workspace>) {
    internal_action_graph(args, workspace).await?;
}
