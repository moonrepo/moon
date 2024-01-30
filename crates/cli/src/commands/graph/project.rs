use crate::commands::graph::utils::{project_graph_repr, respond_to_request, setup_server};
use clap::Args;
use moon::build_project_graph;
use moon_common::Id;
use moon_workspace::Workspace;
use starbase::system;
use starbase_styles::color;

#[derive(Args, Clone, Debug)]
pub struct ProjectGraphArgs {
    #[arg(help = "ID of project to *only* graph")]
    id: Option<Id>,

    #[arg(long, help = "Print the graph in DOT format")]
    dot: bool,

    #[arg(long, help = "Print the graph in JSON format")]
    json: bool,
}

#[system]
pub async fn project_graph(args: ArgsRef<ProjectGraphArgs>, workspace: ResourceMut<Workspace>) {
    let mut project_graph_builder = build_project_graph(workspace).await?;

    if let Some(id) = &args.id {
        project_graph_builder.load(id).await?;
    } else {
        project_graph_builder.load_all().await?;
    }

    let project_graph = project_graph_builder.build().await?;

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
}
