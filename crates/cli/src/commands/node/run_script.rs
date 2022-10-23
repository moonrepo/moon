use crate::helpers::load_workspace;
use moon_error::MoonError;
use moon_project_graph::project_graph::ProjectGraph;
use moon_utils::process::Command;
use std::env;

pub async fn run_script(
    name: &str,
    project: &Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let workspace = load_workspace().await?;
    let mut command = Command::new(
        workspace
            .toolchain
            .node
            .get()?
            .get_package_manager()
            .get_bin_path(),
    );

    command.arg("run").arg(name);

    if let Ok(project_root) = env::var("MOON_PROJECT_ROOT") {
        command.cwd(project_root);
    } else if let Some(project_id) = project {
        let project_graph = ProjectGraph::generate(&workspace).await?;

        command.cwd(project_graph.load(project_id)?.root);
    } else {
        return Err(MoonError::Generic(
            "This command must be ran within the context of a project.".to_owned(),
        )
        .into());
    }

    command.exec_stream_output().await?;

    Ok(())
}
