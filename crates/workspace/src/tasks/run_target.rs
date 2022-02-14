use crate::errors::WorkspaceError;
use crate::workspace::Workspace;
use moon_cache::RunTargetState;
use moon_config::TaskType;
use moon_logger::{color, debug};
use moon_project::{Project, Target, Task};
use moon_toolchain::tools::node::NodeTool;
use moon_toolchain::{get_path_env_var, Tool};
use moon_utils::process::{create_command, exec_command, output_to_string, spawn_command};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::process::Command;
use tokio::sync::RwLock;

async fn create_env_vars(
    workspace: &Workspace,
    project: &Project,
    task: &Task,
) -> Result<HashMap<String, String>, WorkspaceError> {
    let map_path_buf = |path: &Path| String::from(path.to_str().unwrap());
    let mut env_vars = HashMap::new();

    env_vars.insert(
        "MOON_CACHE_DIR".to_owned(),
        map_path_buf(&workspace.cache.dir),
    );
    env_vars.insert("MOON_PROJECT_ID".to_owned(), project.id.clone());
    env_vars.insert("MOON_PROJECT_ROOT".to_owned(), map_path_buf(&project.root));
    env_vars.insert("MOON_PROJECT_SOURCE".to_owned(), project.source.clone());
    env_vars.insert("MOON_RUN_TARGET".to_owned(), task.target.clone());
    env_vars.insert(
        "MOON_TOOLCHAIN_DIR".to_owned(),
        map_path_buf(&workspace.toolchain.dir),
    );
    env_vars.insert(
        "MOON_WORKSPACE_ROOT".to_owned(),
        map_path_buf(&workspace.root),
    );
    env_vars.insert(
        "MOON_WORKING_DIR".to_owned(),
        map_path_buf(&workspace.working_dir),
    );

    // Store runtime data on the file system so that downstream commands can utilize it
    let runfile = workspace
        .cache
        .runfile("projects", &project.id, project)
        .await?;

    env_vars.insert(
        "MOON_PROJECT_RUNFILE".to_owned(),
        map_path_buf(&runfile.path),
    );

    Ok(env_vars)
}

/// Runs a task command through our toolchain's installed Node.js instance.
/// We accomplish this by executing the Node.js binary as a child process,
/// while passing a file path to a package's node module binary (this is the file
/// being executed). We then also pass arguments defined in the task.
/// This would look something like the following:
///
/// ~/.moon/tools/node/1.2.3/bin/node --inspect /path/to/node_modules/.bin/eslint
///     --cache --color --fix --ext .ts,.tsx,.js,.jsx
fn create_node_target_command(
    project: &Project,
    task: &Task,
    node: &NodeTool,
    exec_dir: &Path,
    env_vars: HashMap<String, String>,
) -> Result<Command, WorkspaceError> {
    // Node args
    let package_bin_path = node.find_package_bin_path(&task.command, &project.root)?;
    let mut args = vec![
        // "--inspect", // Enable node inspector
        "--preserve-symlinks",
        package_bin_path.to_str().unwrap(),
    ];

    // Package args
    args.extend(task.args.iter().map(|a| a.as_str()));

    // Create the command
    let mut cmd = create_command(node.get_bin_path());

    cmd.args(&args)
        .current_dir(&exec_dir)
        .env("PATH", get_path_env_var(node.get_bin_dir()))
        .envs(env_vars);

    Ok(cmd)
}

fn create_shell_target_command(
    task: &Task,
    exec_dir: &Path,
    env_vars: HashMap<String, String>,
) -> Command {
    let mut cmd = create_command(&task.command);
    cmd.args(&task.args).current_dir(&exec_dir).envs(env_vars);
    cmd
}

pub async fn run_target(
    workspace: Arc<RwLock<Workspace>>,
    target: &str,
    primary_target: &str,
) -> Result<(), WorkspaceError> {
    debug!(
        target: "moon:task-runner:run-target",
        "Running target {}",
        color::id(target)
    );

    let workspace = workspace.read().await;
    let mut cache = workspace.cache.run_target_state(target).await?;
    let toolchain = &workspace.toolchain;

    // TODO abort early for a cache hit

    // Gather the project and task
    let (project_id, task_id) = Target::parse(target)?;
    let project = workspace.projects.get(&project_id)?;
    let task = project.get_task(&task_id)?;

    // Run the task command as a child process
    let exec_dir = if task.options.run_from_workspace_root {
        &workspace.root
    } else {
        &project.root
    };

    let env_vars = create_env_vars(&workspace, &project, task).await?;
    let mut command = match task.type_of {
        TaskType::Node => {
            create_node_target_command(&project, task, toolchain.get_node(), exec_dir, env_vars)?
        }
        _ => create_shell_target_command(task, exec_dir, env_vars),
    };

    // Run the command as a child process
    let is_primary = target == primary_target;
    let output;

    if is_primary {
        // If this target matches the primary target (the last task to run),
        // then we want to stream the output directly to the parent (inherit mode).
        output = spawn_command(&mut command).await?;
    } else {
        // Otherwise we run the process in the background and write the output
        // once it has completed.
        output = exec_command(&mut command).await?;
    }

    // Update the cache with the result
    cache.item.exit_code = output.status.code().unwrap_or(0);
    cache.item.last_run_time = cache.now_millis();
    cache.item.stderr = output_to_string(&output.stderr);
    cache.item.stdout = output_to_string(&output.stdout);
    cache.save().await?;

    handle_cache_item(target, &cache.item, !is_primary)?;

    Ok(())
}

fn handle_cache_item(target: &str, item: &RunTargetState, log: bool) -> Result<(), WorkspaceError> {
    // Only log when *not* the primary target, or a cache hit
    if log {
        if !item.stderr.is_empty() {
            eprintln!("{}", item.stderr);
        }

        if !item.stdout.is_empty() {
            println!("{}", item.stdout);
        }
    }

    // Return an error if the child process failed
    if item.exit_code != 0 {
        return Err(WorkspaceError::TaskRunnerFailedTarget(target.to_owned()));
    }

    Ok(())
}
