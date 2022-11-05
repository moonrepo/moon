use crate::helpers::load_workspace;
use moon_error::MoonError;
use std::env;

pub async fn run_script(
    name: &str,
    project: &Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let workspace = load_workspace().await?;
    let node = workspace.toolchain.node.get()?;
    let mut command = node.get_package_manager().create_command(&node);

    command.arg("run").arg(name);

    if let Ok(project_root) = env::var("MOON_PROJECT_ROOT") {
        command.cwd(project_root);
    } else if let Some(project_id) = project {
        command.cwd(workspace.projects.load(project_id)?.root);
    } else {
        return Err(MoonError::Generic(
            "This command must be ran within the context of a project.".to_owned(),
        )
        .into());
    }

    command.exec_stream_output().await?;

    Ok(())
}
