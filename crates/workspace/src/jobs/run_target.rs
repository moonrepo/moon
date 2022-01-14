use crate::errors::WorkspaceError;
use crate::workspace::Workspace;
use moon_config::TargetID;
use moon_project::Target;

#[allow(dead_code)]
pub async fn run_target(workspace: &Workspace, target: TargetID) -> Result<(), WorkspaceError> {
    let mut cache = workspace.cache.target_run(target).await?;
    let toolchain = &workspace.toolchain;

    // Gather the project and task
    let (project_id, task_id) = Target::parse(&target);
    let project = workspace.projects.get(project_id)?;
    let task = project.tasks.get(task_id)?;

    // Update the cache with the timestamp
    cache.item.last_run_time = workspace.cache.to_millis(SystemTime::now());
    cache.save().await?;

    Ok(())
}
