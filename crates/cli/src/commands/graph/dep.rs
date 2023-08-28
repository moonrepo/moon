use crate::commands::graph::utils::{dep_graph_repr, respond_to_request, setup_server};
use clap::Args;
use miette::IntoDiagnostic;
use moon::{build_dep_graph, generate_project_graph, load_workspace};
use moon_target::Target;
use starbase::{system, ExecuteArgs};

#[derive(Args, Clone, Debug)]
pub struct DepGraphArgs {
    #[arg(help = "Target to *only* graph")]
    target: Option<String>,

    #[arg(long, help = "Print the graph in DOT format")]
    dot: bool,

    #[arg(long, help = "Print the graph in JSON format")]
    json: bool,
}

#[system]
pub async fn dep_graph(args: StateRef<ExecuteArgs, DepGraphArgs>) {
    let mut workspace = load_workspace().await?;
    let project_graph = generate_project_graph(&mut workspace).await?;
    let mut dep_builder = build_dep_graph(&project_graph);

    // Focus a target and its dependencies/dependents
    if let Some(id) = &args.target {
        let target = Target::parse(id)?;

        dep_builder.run_target(&target, None)?;
        dep_builder.run_dependents_for_target(&target)?;

        // Show all targets and actions
    } else {
        for project in project_graph.get_all_unexpanded() {
            for task in project.tasks.values() {
                dep_builder.run_target(&task.target, None)?;
            }
        }
    }

    let dep_graph = dep_builder.build();

    if args.dot {
        println!("{}", dep_graph.to_dot());

        return Ok(());
    }

    let graph_info = dep_graph_repr(&dep_graph).await;

    if args.json {
        println!("{}", serde_json::to_string(&graph_info).into_diagnostic()?);

        return Ok(());
    }

    let (server, mut tera) = setup_server().await?;
    let url = format!("http://{}", server.server_addr());
    let _ = open::that(&url);

    println!("Started server on {url}");

    for req in server.incoming_requests() {
        respond_to_request(req, &mut tera, &graph_info, "Dependency graph".to_owned())?;
    }
}
