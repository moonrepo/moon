use crate::action::{Action, ActionStatus, Attempt};
use crate::actions::hashing::create_target_hasher;
use crate::errors::WorkspaceError;
use crate::workspace::Workspace;
use moon_cache::RunTargetState;
use moon_config::TaskType;
use moon_logger::{color, debug, trace, warn};
use moon_project::{Project, Target, Task};
use moon_terminal::output::{label_checkpoint, Checkpoint};
use moon_toolchain::{get_path_env_var, Executable};
use moon_utils::process::{join_args, output_to_string, Command, Output};
use moon_utils::{is_ci, is_test_env, path, string_vec, time};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;

const LOG_TARGET: &str = "moon:action:run-target";

async fn create_env_vars(
    workspace: &Workspace,
    project: &Project,
    task: &Task,
) -> Result<HashMap<String, String>, WorkspaceError> {
    let mut env_vars = HashMap::new();

    trace!(
        target: LOG_TARGET,
        "Creating {} environment variables",
        color::shell("MOON_*")
    );

    env_vars.insert(
        "MOON_CACHE_DIR".to_owned(),
        path::path_to_string(&workspace.cache.dir)?,
    );
    env_vars.insert("MOON_PROJECT_ID".to_owned(), project.id.clone());
    env_vars.insert(
        "MOON_PROJECT_ROOT".to_owned(),
        path::path_to_string(&project.root)?,
    );
    env_vars.insert("MOON_PROJECT_SOURCE".to_owned(), project.source.clone());
    env_vars.insert("MOON_TARGET".to_owned(), task.target.clone());
    env_vars.insert(
        "MOON_TOOLCHAIN_DIR".to_owned(),
        path::path_to_string(&workspace.toolchain.dir)?,
    );
    env_vars.insert(
        "MOON_WORKSPACE_ROOT".to_owned(),
        path::path_to_string(&workspace.root)?,
    );
    env_vars.insert(
        "MOON_WORKING_DIR".to_owned(),
        path::path_to_string(&workspace.working_dir)?,
    );

    // Store runtime data on the file system so that downstream commands can utilize it
    let runfile = workspace.cache.create_runfile(&project.id, project).await?;

    env_vars.insert(
        "MOON_PROJECT_RUNFILE".to_owned(),
        path::path_to_string(&runfile.path)?,
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
            cmd = node.get_npm().get_bin_path();
        }
        "pnpm" => {
            cmd = node.get_pnpm().unwrap().get_bin_path();
        }
        "yarn" => {
            cmd = node.get_yarn().unwrap().get_bin_path();
        }
        bin => {
            let bin_path = node.find_package_bin(bin, &project.root)?;

            args.extend(create_node_options(task));
            args.push(path::path_to_string(&bin_path)?);
        }
    };

    // Create the command
    let mut command = Command::new(cmd);

    command.args(&args).args(&task.args).envs(&task.env).env(
        "PATH",
        get_path_env_var(node.get_bin_path().parent().unwrap()),
    );

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
    use moon_lang_node::node;

    let node = workspace.toolchain.get_node();

    let cmd = match task.command.as_str() {
        "node" => node.get_bin_path().clone(),
        "npm" => node.get_npm().get_bin_path().clone(),
        "pnpm" => node.get_pnpm().unwrap().get_bin_path().clone(),
        "yarn" => node.get_yarn().unwrap().get_bin_path().clone(),
        bin => node.find_package_bin(bin, &project.root)?,
    };

    // Create the command
    let mut command = Command::new(cmd);

    command
        .args(&task.args)
        .envs(&task.env)
        .env(
            "PATH",
            get_path_env_var(node.get_bin_path().parent().unwrap()),
        )
        .env(
            "NODE_OPTIONS",
            node::extend_node_options_env_var(create_node_options(task).join(" ")),
        );

    Ok(command)
}

#[cfg(not(windows))]
fn create_system_target_command(task: &Task, _cwd: &Path) -> Command {
    let mut cmd = Command::new(&task.command);
    cmd.args(&task.args).envs(&task.env);
    cmd
}

#[cfg(windows)]
fn create_system_target_command(task: &Task, cwd: &Path) -> Command {
    use moon_utils::process::is_windows_script;

    let mut cmd = Command::new(&task.command);

    for arg in &task.args {
        // cmd.exe requires an absolute path to batch files
        if is_windows_script(arg) {
            cmd.arg(cwd.join(arg));
        } else {
            cmd.arg(arg);
        }
    }

    cmd.envs(&task.env);
    cmd
}

async fn create_target_command(
    workspace: &Workspace,
    project: &Project,
    task: &Task,
) -> Result<Command, WorkspaceError> {
    let working_dir = if task.options.run_from_workspace_root {
        &workspace.root
    } else {
        &project.root
    };

    debug!(
        target: LOG_TARGET,
        "Creating {} command (in working directory {})",
        color::target(&task.target),
        color::path(working_dir)
    );

    let mut command = match task.type_of {
        TaskType::Node => create_node_target_command(workspace, project, task)?,
        _ => create_system_target_command(task, working_dir),
    };

    let env_vars = create_env_vars(workspace, project, task).await?;

    command
        .cwd(working_dir)
        .envs(env_vars)
        // We need to handle non-zero's manually
        .no_error_on_failure();

    Ok(command)
}

pub async fn run_target(
    workspace: Arc<RwLock<Workspace>>,
    action: &mut Action,
    target_id: &str,
    primary_target: &str,
    passthrough_args: &[String],
) -> Result<ActionStatus, WorkspaceError> {
    debug!(
        target: LOG_TARGET,
        "Running target {}",
        color::id(target_id)
    );

    let workspace = workspace.read().await;
    let mut cache = workspace.cache.cache_run_target_state(target_id).await?;

    // Gather the project and task
    let is_primary = primary_target == target_id;
    let (project_id, task_id) = Target::parse(target_id)?.ids()?;
    let project = workspace.projects.load(&project_id)?;
    let task = project.get_task(&task_id)?;

    // Abort early if this build has already been cached/hashed
    let hasher = create_target_hasher(&workspace, &project, task, passthrough_args).await?;
    let hash = hasher.to_hash();

    debug!(
        target: LOG_TARGET,
        "Generated hash {} for target {}",
        color::symbol(&hash),
        color::id(target_id)
    );

    if cache.item.hash == hash {
        debug!(
            target: LOG_TARGET,
            "Hash exists for {}, aborting run",
            color::id(target_id),
        );

        println!(
            "{} {}",
            label_checkpoint(target_id, Checkpoint::Pass),
            color::muted("(cached)")
        );

        print_cache_item(&cache.item);

        return Ok(ActionStatus::Cached);
    }

    // Build the command to run based on the task
    let mut command = create_target_command(&workspace, &project, task).await?;
    command.args(passthrough_args);

    if workspace
        .config
        .action_runner
        .inherit_colors_for_piped_tasks
    {
        command.inherit_colors();
    }

    // Run the command as a child process and capture its output.
    // If the process fails and `retry_count` is greater than 0,
    // attempt the process again in case it passes.
    let attempt_total = task.options.retry_count + 1;
    let mut attempt_index = 1;
    let mut attempts = vec![];
    let is_real_ci = is_ci() && !is_test_env();
    let stream_output = is_primary || is_real_ci;
    let output;

    loop {
        let mut attempt = Attempt::new(attempt_index);

        let possible_output = if stream_output {
            // Print label *before* output is streamed since it may stay open forever,
            // or it may use ANSI escape codes to alter the terminal.
            print_target_label(target_id, &attempt, attempt_total, Checkpoint::Pass);
            print_target_command(&workspace, &project, task, passthrough_args);

            // If this target matches the primary target (the last task to run),
            // then we want to stream the output directly to the parent (inherit mode).
            command
                .exec_stream_and_capture_output(if is_real_ci { Some(target_id) } else { None })
                .await
        } else {
            print_target_label(target_id, &attempt, attempt_total, Checkpoint::Start);
            print_target_command(&workspace, &project, task, passthrough_args);

            // Otherwise we run the process in the background and write the output
            // once it has completed.
            command.exec_capture_output().await
        };

        attempt.done();

        match possible_output {
            // zero and non-zero exit codes
            Ok(out) => {
                if stream_output {
                    handle_streamed_output(target_id, &attempt, attempt_total, &out);
                } else {
                    handle_captured_output(target_id, &attempt, attempt_total, &out);
                }

                attempts.push(attempt);

                if out.status.success() {
                    output = out;
                    break;
                } else if attempt_index >= attempt_total {
                    return Err(WorkspaceError::Moon(command.output_to_error(&out, false)));
                } else {
                    attempt_index += 1;

                    warn!(
                        target: LOG_TARGET,
                        "Target {} failed, running again with attempt {}",
                        color::target(target_id),
                        attempt_index
                    );
                }
            }
            // process itself failed
            Err(error) => {
                return Err(WorkspaceError::Moon(error));
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

    // Save the new hash
    workspace.cache.save_hash(&hash, &hasher).await?;
    action.attempts = Some(attempts);

    // Write the cache with the result and output
    cache.item.exit_code = output.status.code().unwrap_or(0);
    cache.item.hash = hash;
    cache.item.last_run_time = cache.now_millis();
    cache.item.stderr = output_to_string(&output.stderr);
    cache.item.stdout = output_to_string(&output.stdout);
    cache.save().await?;

    Ok(ActionStatus::Passed)
}

fn print_target_label(target: &str, attempt: &Attempt, attempt_total: u8, checkpoint: Checkpoint) {
    let failed = matches!(checkpoint, Checkpoint::Fail);
    let mut label = label_checkpoint(target, checkpoint);
    let mut comments = vec![];

    if attempt.index > 1 {
        comments.push(format!("{}/{}", attempt.index, attempt_total));
    }

    if let Some(duration) = attempt.duration {
        comments.push(time::elapsed(duration));
    }

    if !comments.is_empty() {
        let metadata = color::muted(&format!("({})", comments.join(", ")));

        label = format!("{} {}", label, metadata);
    };

    if failed {
        eprintln!("{}", label);
    } else {
        println!("{}", label);
    }
}

fn print_target_command(
    workspace: &Workspace,
    project: &Project,
    task: &Task,
    passthrough_args: &[String],
) {
    if !workspace.config.action_runner.log_running_command {
        return;
    }

    let mut args = vec![];
    args.extend(&task.args);
    args.extend(passthrough_args);

    let command_line = if args.is_empty() {
        task.command.clone()
    } else {
        format!("{} {}", task.command, join_args(args))
    };

    let working_dir = if task.options.run_from_workspace_root || project.root == workspace.root {
        String::from("workspace")
    } else {
        format!(
            ".{}{}",
            std::path::MAIN_SEPARATOR,
            project
                .root
                .strip_prefix(&workspace.root)
                .unwrap()
                .to_string_lossy(),
        )
    };

    let suffix = format!("(in {})", working_dir);
    let message = format!("{} {}", command_line, color::muted(&suffix));

    println!("{}", color::muted_light(&message));
}

fn print_cache_item(item: &RunTargetState) {
    if !item.stderr.is_empty() {
        eprintln!("{}", item.stderr.trim());
        eprintln!();
    }

    if !item.stdout.is_empty() {
        println!("{}", item.stdout.trim());
        println!();
    }
}

fn print_output_std(output: &Output) {
    let stderr = output_to_string(&output.stderr);
    let stdout = output_to_string(&output.stdout);

    if !stderr.is_empty() {
        eprintln!("{}", stderr.trim());
        eprintln!();
    }

    if !stdout.is_empty() {
        println!("{}", stdout.trim());
        println!();
    }
}

// Print label *after* output has been captured, so parallel tasks
// aren't intertwined and the labels align with the output.
fn handle_captured_output(target_id: &str, attempt: &Attempt, attempt_total: u8, output: &Output) {
    print_target_label(
        target_id,
        attempt,
        attempt_total,
        if output.status.success() {
            Checkpoint::Pass
        } else {
            Checkpoint::Fail
        },
    );

    print_output_std(output);
}

// Only print the label when the process has failed,
// as the actual output has already been streamed to the console.
fn handle_streamed_output(target_id: &str, attempt: &Attempt, attempt_total: u8, output: &Output) {
    if !output.status.success() {
        print_target_label(target_id, attempt, attempt_total, Checkpoint::Fail);
    }
}
