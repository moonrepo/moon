use clap::Args;
use moon::{build_project_graph, load_workspace_with_toolchain};
use moon_common::Id;
use moon_config::PlatformType;
use moon_node_tool::NodeTool;
use moon_platform::PlatformManager;
use starbase::AppResult;
use std::env;

#[derive(Args, Debug)]
pub struct RunScriptArgs {
    #[arg(help = "Name of the script")]
    name: String,

    #[arg(long, help = "ID of project to run in")]
    project: Option<Id>,
}

pub async fn run_script(args: RunScriptArgs) -> AppResult {
    let mut workspace = load_workspace_with_toolchain().await?;
    let node = PlatformManager::read()
        .get(PlatformType::Node)?
        .get_tool()?
        .as_any()
        .downcast_ref::<NodeTool>()
        .unwrap();

    let mut command = node.get_package_manager().create_command(node)?;

    command.arg("run").arg(&args.name);

    // Use the env var provided by our task runner
    if let Ok(project_root) = env::var("MOON_PROJECT_ROOT") {
        command.cwd(project_root);

        // Otherwise try and find the project in the graph
    } else if let Some(project_id) = &args.project {
        let mut project_graph = build_project_graph(&mut workspace).await?;
        project_graph.load(project_id).await?;

        command.cwd(&project_graph.build().await?.get(project_id)?.root);

        // This should rarely happen...
    } else {
        return Err(miette::miette!(
            "This command must be ran within the context of a project.",
        ));
    }

    command.create_async().exec_stream_output().await?;

    Ok(())
}
