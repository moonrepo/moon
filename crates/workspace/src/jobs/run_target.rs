use crate::errors::WorkspaceError;
use crate::workspace::Workspace;
use moon_config::TaskType;
use moon_logger::debug;
use moon_project::{Project, Target, Task};
use moon_toolchain::tools::node::NodeTool;
use moon_toolchain::Tool;
use moon_utils::process::{exec_bin_in_dir, exec_command_in_dir, output_to_string};
use std::path::Path;
use std::process::Output;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Runs a task command through our toolchain's installed Node.js instance.
/// We accomplish this by executing the Node.js binary as a child process,
/// while passing a file path to a package's node module binary (this is the file
/// being executed). We then also pass arguments defined in the task.
/// This would look something like the following:
///
/// ~/.moon/tools/node/1.2.3/bin/node --inspect /path/to/node_modules/.bin/eslint
///     --cache --color --fix --ext .ts,.tsx,.js,.jsx
async fn run_node_target(
    project: &Project,
    task: &Task,
    node: &NodeTool,
    exec_dir: &Path,
) -> Result<Output, WorkspaceError> {
    let command_path = node.find_package_bin_path(&task.command, &project.root)?;

    let mut args = vec![
        // "--inspect", // Enable node inspector
        "--preserve-symlinks",
        command_path.to_str().unwrap(),
    ];

    args.extend(task.args.iter().map(|a| a.as_str()));

    Ok(exec_bin_in_dir(node.get_bin_path(), args, exec_dir).await?)
}

async fn run_shell_target(task: &Task, exec_dir: &Path) -> Result<Output, WorkspaceError> {
    Ok(exec_command_in_dir(
        &task.command,
        task.args.iter().map(|a| a.as_str()).collect(),
        exec_dir,
    )
    .await?)
}

pub async fn run_target(
    workspace: Arc<RwLock<Workspace>>,
    target: &str,
) -> Result<(), WorkspaceError> {
    debug!(
        target: "moon:orchestrator:run-target",
        "Running target {}",
        target
    );

    let workspace = workspace.read().await;
    let mut cache = workspace.cache.run_target_state(target).await?;
    let toolchain = &workspace.toolchain;

    // TODO abort early for a cache hit

    // Gather the project and task
    let (project_id, task_id) = Target::parse(target)?;
    let project = workspace.projects.get(&project_id)?;
    let task = project.tasks.get(&task_id).unwrap();

    // Run the task command as a child process
    let exec_dir = if task.options.run_from_workspace_root {
        &workspace.root
    } else {
        &project.root
    };

    let output = match task.type_of {
        TaskType::Node => run_node_target(&project, task, toolchain.get_node(), exec_dir).await?,
        _ => run_shell_target(task, exec_dir).await?,
    };

    // Update the cache with the result
    cache.item.exit_code = output.status.code().unwrap_or(0);
    cache.item.last_run_time = cache.now_millis();
    cache.item.stderr = output_to_string(output.stderr);
    cache.item.stdout = output_to_string(output.stdout);
    cache.save().await?;

    Ok(())
}
