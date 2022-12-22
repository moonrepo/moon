use crate::helpers::AnyError;
use moon::{build_project_graph, load_workspace_with_toolchain};
use moon_error::MoonError;
use moon_node_tool::NodeTool;
use std::env;

pub async fn run_script(name: &str, project: &Option<String>) -> Result<(), AnyError> {
    let mut workspace = load_workspace_with_toolchain().await?;
    let node = workspace.toolchain.node.get::<NodeTool>()?;
    let mut command = node.get_package_manager().create_command(node)?;

    command.arg("run").arg(name);

    // Use the env var provided by our task runner
    if let Ok(project_root) = env::var("MOON_PROJECT_ROOT") {
        command.cwd(project_root);

        // Otherwise try and find the project in the graph
    } else if let Some(project_id) = project {
        let mut project_graph = build_project_graph(&mut workspace)?;
        project_graph.load(project_id)?;

        command.cwd(&project_graph.build().get(project_id)?.root);

        // This should rarely happen...
    } else {
        return Err(MoonError::Generic(
            "This command must be ran within the context of a project.".to_owned(),
        )
        .into());
    }

    command.exec_stream_output().await?;

    Ok(())
}
