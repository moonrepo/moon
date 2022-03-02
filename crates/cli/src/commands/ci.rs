use moon_logger::{color, debug};
use moon_project::Target;
use moon_workspace::DepGraph;
use moon_workspace::Workspace;
use std::collections::HashSet;
use std::path::PathBuf;

const TARGET: &str = "moon:ci";

#[allow(dead_code)]
pub async fn ci() -> Result<(), Box<dyn std::error::Error>> {
    let workspace = Workspace::load().await?;
    let vcs = workspace.detect_vcs();

    // Gather a list of files that have been modified between branches
    let branch = vcs.get_local_branch().await?;
    let touched_files_map = if vcs.is_default_branch(&branch) {
        // On master/main, so compare against master -1 commit
        vcs.get_touched_files_against_branch("HEAD", 1).await?
    } else {
        // On a branch, so compare branch against master/main
        vcs.get_touched_files_against_branch(&branch, 0).await?
    };
    let touched_files: HashSet<PathBuf> = touched_files_map
        .all
        .iter()
        .map(|f| workspace.root.join(f))
        .collect();

    // Generate a dependency graph for all the targets that need to be ran
    let mut dep_graph = DepGraph::default();

    for project_id in workspace.projects.ids() {
        let project = workspace.projects.load(&project_id)?;

        for (task_id, task) in project.tasks {
            let target = Target::format(&project_id, &task_id)?;

            // Besides touched files, we should only run a target if they
            // have outputs, or the `run_in_ci` option is true
            if !task.outputs.is_empty() || task.options.run_in_ci {
                dep_graph.run_target_if_touched(&target, &touched_files, &workspace.projects)?;
            } else {
                debug!(
                    target: TARGET,
                    "Not running target {} because it either has no `outputs` or `runInCi` is false",
                    color::target(&target),
                );
            }
        }
    }

    Ok(())
}
