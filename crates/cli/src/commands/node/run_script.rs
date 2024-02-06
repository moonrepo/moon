use clap::Args;
use miette::miette;
use moon::build_project_graph;
use moon_common::Id;
use moon_config::PlatformType;
use moon_node_tool::NodeTool;
use moon_platform::PlatformManager;
use moon_workspace::Workspace;
use starbase::system;
use starbase_styles::color;
use std::env;
use tracing::warn;

#[derive(Args, Clone, Debug)]
pub struct RunScriptArgs {
    #[arg(help = "Name of the script")]
    name: String,

    #[arg(long, help = "ID of project to run in")]
    project: Option<Id>,
}

#[system]
pub async fn run_script(args: ArgsRef<RunScriptArgs>, workspace: ResourceMut<Workspace>) {
    let node = PlatformManager::read()
        .get(PlatformType::Node)?
        .get_tool()?
        .as_any()
        .downcast_ref::<NodeTool>()
        .unwrap();

    warn!(
        "The command {} is deprecated, update your task to run through a package manager instead. For example, {}.",
        color::shell("moon node run-script"),
        color::shell(format!("{} run {}", node.config.package_manager, &args.name)),
    );

    let mut command = node.get_package_manager().create_command(node)?;

    command.arg("run").arg(&args.name);

    // Use the env var provided by our task runner
    if let Ok(project_root) = env::var("MOON_PROJECT_ROOT") {
        command.cwd(project_root);

        // Otherwise try and find the project in the graph
    } else if let Some(project_id) = &args.project {
        let mut project_graph = build_project_graph(workspace).await?;
        project_graph.load(project_id).await?;

        command.cwd(&project_graph.build().await?.get(project_id)?.root);

        // This should rarely happen...
    } else {
        return Err(miette!(
            code = "moon::node::run_script",
            "This command must be ran within the context of a project.",
        ));
    }

    command.create_async().exec_stream_output().await?;
}
