use crate::errors::ActionRunnerError;
use moon_project::{Project, Task};
use moon_toolchain::{get_path_env_var, Executable};
use moon_utils::process::Command;
use moon_utils::{path, string_vec};
use moon_workspace::Workspace;

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
#[track_caller]
pub fn create_node_target_command(
    workspace: &Workspace,
    project: &Project,
    task: &Task,
) -> Result<Command, ActionRunnerError> {
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
#[track_caller]
pub fn create_node_target_command(
    workspace: &Workspace,
    project: &Project,
    task: &Task,
) -> Result<Command, ActionRunnerError> {
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
            node::extend_node_options_env_var(&create_node_options(task).join(" ")),
        );

    Ok(command)
}
