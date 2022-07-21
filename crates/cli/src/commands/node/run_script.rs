use moon_error::MoonError;
use moon_utils::process::Command;
use moon_workspace::Workspace;
use std::env;

pub async fn run_script(name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let workspace = Workspace::load().await?;

    let project_root = env::var("MOON_PROJECT_ROOT").map_err(|_| {
        MoonError::Generic("This command must be ran within the context of a project.".to_owned())
    })?;

    let mut command = Command::new(workspace.config.node.package_manager.get_bin_name());

    command
        .arg("run")
        .arg(name)
        .cwd(project_root)
        .exec_stream_output()
        .await?;

    Ok(())
}
