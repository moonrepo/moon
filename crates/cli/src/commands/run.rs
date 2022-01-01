use moon_project::{AffectedFiles, TargetID, TaskGraph};
use moon_workspace::{TouchedFiles, Workspace};

// TODO: Filter touched files based on their last modified time
fn get_affected_files(workspace: &Workspace, touched_files: &TouchedFiles) -> AffectedFiles {}

pub async fn run(target: TargetID) -> Result<(), Box<dyn std::error::Error>> {
    let workspace = Workspace::load()?;
    let touched_files = workspace.vcs.get_touched_files()?;

    let graph = TaskGraph::new(&workspace.projects, target, touched_files.all);

    workspace.toolchain.setup().await?;

    Ok(())
}
