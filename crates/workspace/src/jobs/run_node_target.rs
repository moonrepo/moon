use crate::errors::WorkspaceError;
use crate::workspace::Workspace;
use moon_config::TargetID;
use moon_project::Target;
use moon_toolchain::Tool;
use moon_utils::{exec_bin_in_dir, output_to_string};

/// Runs a task command through our toolchain's installed Node.js instance.
/// We accomplish this by executing the Node.js binary as a child process,
/// while passing a file path to a package's node module binary (this is the file
/// being executed). We then also pass arguments defined in the task.
/// This would look something like the following:
///
/// ~/.moon/tools/node/1.2.3/bin/node --inspect /path/to/node_modules/.bin/eslint
///     --cache --color --fix --ext .ts,.tsx,.js,.jsx
///
#[allow(dead_code)]
pub async fn run_node_target(
    workspace: &Workspace,
    target: TargetID,
) -> Result<(), WorkspaceError> {
    let mut cache = workspace.cache.target_run_state(&target).await?;
    let toolchain = &workspace.toolchain;
    let node = toolchain.get_node();

    // TODO abort early for a cache hit

    // Gather the project and task
    let (project_id, task_id) = Target::parse(&target)?;
    let project = workspace.projects.get(&project_id)?;
    let task = project.tasks.get(&task_id).unwrap();

    // Gather arguments to pass on the command line
    let command_path = node.find_package_bin_path(&task.command, &project.root)?;

    let mut args = vec![
        // "--inspect", // Enable node inspector
        "--preserve-symlinks",
        command_path.to_str().unwrap(),
    ];

    args.extend(task.args.iter().map(|a| a.as_str()));

    // Run the task command as a process
    let output = exec_bin_in_dir(node.get_bin_path(), args, &project.root).await?;

    // Update the cache with the result
    cache.item.exit_code = output.status.code().unwrap_or(0);
    cache.item.last_run_time = cache.now_millis();
    cache.item.stderr = output_to_string(output.stderr);
    cache.item.stdout = output_to_string(output.stdout);
    cache.save().await?;

    Ok(())
}
