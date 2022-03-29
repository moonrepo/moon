use crate::errors::WorkspaceError;
use crate::task_result::TaskResultStatus;
use crate::tasks::hashing::create_target_hasher;
use crate::workspace::Workspace;
use moon_cache::RunTargetState;
use moon_config::TaskType;
use moon_logger::{color, debug};
use moon_project::{Project, Target, Task};
use moon_terminal::output::{label_run_target, label_run_target_failed};
use moon_toolchain::{get_path_env_var, Tool};
use moon_utils::process::{create_command, exec_command, output_to_string, spawn_command};
use moon_utils::{fs, string_vec};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::process::Command;
use tokio::sync::RwLock;

const TARGET: &str = "moon:task-runner:run-target";

async fn create_env_vars(
    workspace: &Workspace,
    project: &Project,
    task: &Task,
) -> Result<HashMap<String, String>, WorkspaceError> {
    let mut env_vars = HashMap::new();

    env_vars.insert(
        "MOON_CACHE_DIR".to_owned(),
        fs::path_to_string(&workspace.cache.dir)?,
    );
    env_vars.insert("MOON_PROJECT_ID".to_owned(), project.id.clone());
    env_vars.insert(
        "MOON_PROJECT_ROOT".to_owned(),
        fs::path_to_string(&project.root)?,
    );
    env_vars.insert("MOON_PROJECT_SOURCE".to_owned(), project.source.clone());
    env_vars.insert("MOON_RUN_TARGET".to_owned(), task.target.clone());
    env_vars.insert(
        "MOON_TOOLCHAIN_DIR".to_owned(),
        fs::path_to_string(&workspace.toolchain.dir)?,
    );
    env_vars.insert(
        "MOON_WORKSPACE_ROOT".to_owned(),
        fs::path_to_string(&workspace.root)?,
    );
    env_vars.insert(
        "MOON_WORKING_DIR".to_owned(),
        fs::path_to_string(&workspace.working_dir)?,
    );

    // Store runtime data on the file system so that downstream commands can utilize it
    let runfile = workspace.cache.create_runfile(&project.id, project).await?;

    env_vars.insert(
        "MOON_PROJECT_RUNFILE".to_owned(),
        fs::path_to_string(&runfile.path)?,
    );

    Ok(env_vars)
}

fn create_node_options(task: &Task) -> Vec<String> {
    string_vec![
        // "--inspect", // Enable node inspector
        "--preserve-symlinks",
        "--title",
        &task.target,
        "--unhandled-rejections",
        "throw",
    ]
}

/// Runs a task command through our toolchain's installed Node.js instance.
/// We accomplish this by executing the Node.js binary as a child process,
/// while passing a file path to a package's node module binary (this is the file
/// being executed). We then also pass arguments defined in the task.
/// This would look something like the following:
///
/// ~/.moon/tools/node/1.2.3/bin/node --inspect /path/to/node_modules/.bin/eslint
///     --cache --color --fix --ext .ts,.tsx,.js,.jsx
#[cfg(not(windows))]
fn create_node_target_command(
    workspace: &Workspace,
    project: &Project,
    task: &Task,
) -> Result<Command, WorkspaceError> {
    let node = workspace.toolchain.get_node();
    let mut cmd = node.get_bin_path();
    let mut args = vec![];

    match task.command.as_str() {
        "node" => {
            args.extend(create_node_options(task));
        }
        "npm" => {
            cmd = workspace.toolchain.get_npm().get_bin_path();
        }
        "pnpm" => {
            cmd = workspace.toolchain.get_pnpm().unwrap().get_bin_path();
        }
        "yarn" => {
            cmd = workspace.toolchain.get_yarn().unwrap().get_bin_path();
        }
        bin => {
            let bin_path = node.find_package_bin_path(bin, &project.root)?;

            args.extend(create_node_options(task));
            args.push(fs::path_to_string(&bin_path)?);
        }
    };

    args.extend(task.args.clone());

    // Create the command
    let mut command = create_command(cmd);

    command
        .args(&args)
        .envs(&task.env)
        .env("PATH", get_path_env_var(node.get_bin_dir()));

    Ok(command)
}

/// Windows works quite differently than other systems, so we cannot do the above.
/// On Windows, the package binary is a ".cmd" file, which means it needs to run
/// through "cmd.exe" and not "node.exe". Because of this, the order of operations
/// is switched, and "node.exe" is detected through the `PATH` env var.
#[cfg(windows)]
fn create_node_target_command(
    workspace: &Workspace,
    project: &Project,
    task: &Task,
) -> Result<Command, WorkspaceError> {
    let node = workspace.toolchain.get_node();

    let cmd = match task.command.as_str() {
        "node" => node.get_bin_path(),
        "npm" => workspace.toolchain.get_npm().get_bin_path(),
        "pnpm" => workspace.toolchain.get_pnpm().unwrap().get_bin_path(),
        "yarn" => workspace.toolchain.get_yarn().unwrap().get_bin_path(),
        bin => node.find_package_bin_path(bin, &project.root)?,
    };

    // Create the command
    let mut command = create_command(package_bin_path);

    command
        .args(&task.args)
        .envs(&task.env)
        .env("PATH", get_path_env_var(node.get_bin_dir()))
        .env("NODE_OPTIONS", create_node_options(task).join(" "));

    Ok(command)
}

fn create_shell_target_command(task: &Task) -> Command {
    let mut cmd = create_command(&task.command);
    cmd.args(&task.args);
    cmd
}

async fn create_target_command(
    workspace: &Workspace,
    project: &Project,
    task: &Task,
) -> Result<Command, WorkspaceError> {
    let exec_dir = if task.options.run_from_workspace_root {
        &workspace.root
    } else {
        &project.root
    };

    let env_vars = create_env_vars(workspace, project, task).await?;

    let mut command = match task.type_of {
        TaskType::Node => create_node_target_command(workspace, project, task)?,
        _ => create_shell_target_command(task),
    };

    command.current_dir(&exec_dir).envs(env_vars);

    Ok(command)
}

pub async fn run_target(
    workspace: Arc<RwLock<Workspace>>,
    target: &str,
    primary_target: &str,
    passthrough_args: &[String],
) -> Result<TaskResultStatus, WorkspaceError> {
    debug!(target: TARGET, "Running target {}", color::id(target));

    let workspace = workspace.read().await;
    let mut cache = workspace.cache.cache_run_target_state(target).await?;

    // Gather the project and task
    let is_primary = primary_target == target;
    let (project_id, task_id) = Target::parse(target)?;
    let project = workspace.projects.load(&project_id)?;
    let task = project.get_task(&task_id)?;

    // Abort early if this build has already been cached/hashed
    let hasher = create_target_hasher(&workspace, &project, task).await?;
    let hash = hasher.to_hash();

    if cache.item.hash == hash {
        print_target_label(target, "(cached)", cache.item.exit_code != 0);
        print_cache_item(&cache.item, true);

        return Ok(TaskResultStatus::Cached);
    }

    // Build the command to run based on the task
    let mut command = create_target_command(&workspace, &project, task).await?;

    if is_primary && !passthrough_args.is_empty() {
        command.args(passthrough_args);
    }

    // Run the command as a child process and capture its output.
    // If the process fails and `retry_count` is greater than 0,
    // attempt the process again in case it passes.
    let attempt_count = task.options.retry_count + 1;
    let mut attempt = 1;
    let output;

    loop {
        let possible_output;
        let attempt_comment = if attempt == 1 {
            String::new()
        } else {
            format!("(attempt {} of {})", attempt, attempt_count)
        };

        if is_primary {
            // Print label *before* output is streamed since it may stay open forever,
            // or use ANSI escape codes to alter the terminal.
            print_target_label(target, &attempt_comment, false);

            // If this target matches the primary target (the last task to run),
            // then we want to stream the output directly to the parent (inherit mode).
            possible_output = spawn_command(&mut command).await;
        } else {
            // Otherwise we run the process in the background and write the output
            // once it has completed.
            possible_output = exec_command(&mut command).await;

            // Print label *after* output has been captured, so parallel tasks
            // aren't intertwined and the labels align with the output.
            print_target_label(target, &attempt_comment, possible_output.is_err());
        };

        match possible_output {
            Ok(o) => {
                output = o;
                break;
            }
            Err(e) => {
                if attempt >= attempt_count {
                    return Err(WorkspaceError::Moon(e));
                } else {
                    attempt += 1;

                    debug!(
                        target: TARGET,
                        "Target {} failed, running again with attempt {}",
                        color::target(target),
                        attempt
                    );
                }
            }
        }
    }

    // Hard link outputs to the `.moon/cache/out` folder and to the cloud,
    // so that subsequent builds are faster, and any local outputs
    // can be rehydrated easily.
    for output_path in &task.output_paths {
        workspace
            .cache
            .link_task_output_to_out(&hash, &project.root, output_path)
            .await?;
    }

    // Delete the old hash
    if !cache.item.hash.is_empty() && cache.item.hash != hash {
        workspace.cache.delete_hash(&cache.item.hash).await?;
    }

    // Save the new hash
    workspace.cache.save_hash(&hash, &hasher).await?;

    // Write the cache with the result and output
    cache.item.exit_code = output.status.code().unwrap_or(0);
    cache.item.hash = hash;
    cache.item.last_run_time = cache.now_millis();
    cache.item.stderr = output_to_string(&output.stderr);
    cache.item.stdout = output_to_string(&output.stdout);
    cache.save().await?;

    print_cache_item(&cache.item, !is_primary);

    Ok(TaskResultStatus::Passed)
}

fn print_target_label(target: &str, comment: &str, failed: bool) {
    let label = if failed {
        label_run_target_failed(target)
    } else {
        label_run_target(target)
    };

    if comment.is_empty() {
        println!("{}", label);
    } else {
        println!("{} {}", label, color::muted(comment));
    }
}

fn print_cache_item(item: &RunTargetState, log: bool) {
    // Only log when *not* the primary target, or a cache hit
    if log {
        if !item.stderr.is_empty() {
            eprintln!("{}", item.stderr.trim());
            eprintln!();
        }

        if !item.stdout.is_empty() {
            println!("{}", item.stdout.trim());
            println!();
        }
    }
}
