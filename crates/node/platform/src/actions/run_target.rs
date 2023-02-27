use crate::target_hasher::NodeTargetHasher;
use moon_action_context::{ActionContext, ProfileType};
use moon_config::{HasherConfig, HasherOptimization, NodeConfig, NodePackageManager};
use moon_error::MoonError;
use moon_logger::{color, trace};
use moon_node_lang::{
    node::{self, BinFile},
    PackageJson,
};
use moon_node_tool::NodeTool;
use moon_project::Project;
use moon_task::Task;
use moon_tool::{get_path_env_var, DependencyManager, Tool, ToolError};
use moon_utils::{get_cache_dir, process::Command};
use moon_utils::{path, string_vec};
use proto::Installable;
use rustc_hash::FxHashMap;
use std::path::Path;

const LOG_TARGET: &str = "moon:node-platform:run-target";

fn create_node_options(
    node_config: &NodeConfig,
    context: &ActionContext,
    task: &Task,
) -> Result<Vec<String>, MoonError> {
    let mut options = string_vec![
        // "--inspect", // Enable node inspector
        "--title",
        &task.target.id,
    ];

    options.extend(node_config.bin_exec_args.to_owned());

    if let Some(profile) = &context.profile {
        let prof_dir = get_cache_dir()
            .join("states")
            .join(task.target.id.replace(':', "/"));

        match profile {
            ProfileType::Cpu => {
                trace!(
                    target: LOG_TARGET,
                    "Writing CPU profile for {} to {}",
                    color::target(&task.target),
                    color::path(&prof_dir)
                );

                options.extend(string_vec![
                    "--cpu-prof",
                    "--cpu-prof-name",
                    "snapshot.cpuprofile",
                    "--cpu-prof-dir",
                    path::to_string(&prof_dir)?
                ]);
            }
            ProfileType::Heap => {
                trace!(
                    target: LOG_TARGET,
                    "Writing heap profile for {} to {}",
                    color::target(&task.target),
                    color::path(&prof_dir)
                );

                options.extend(string_vec![
                    "--heap-prof",
                    "--heap-prof-name",
                    "snapshot.heapprofile",
                    "--heap-prof-dir",
                    path::to_string(&prof_dir)?
                ]);
            }
        }
    }

    Ok(options)
}

fn find_package_bin(
    command: &mut Command,
    node_options: &[String],
    starting_dir: &Path,
    working_dir: &Path,
    bin_name: &str,
) -> Result<Option<Command>, ToolError> {
    let possible_bin_path = match node::find_package_bin(starting_dir, bin_name)? {
        Some(bin) => Ok(bin),
        None => Err(ToolError::MissingBinary(bin_name.to_owned())),
    };

    match possible_bin_path? {
        // Rust, Go
        BinFile::Binary(bin_path) => {
            return Ok(Some(Command::new(bin_path)));
        }
        // JavaScript
        BinFile::Script(bin_path) => {
            command.args(node_options);
            command.arg(path::to_string(
                path::relative_from(bin_path, working_dir).unwrap(),
            )?);
        }
    };

    Ok(None)
}

fn prepare_target_command(
    command: &mut Command,
    context: &ActionContext,
    task: &Task,
    node_config: &NodeConfig,
) -> Result<(), ToolError> {
    command.args(&task.args).envs(&task.env);

    // This functionality mimics what pnpm's "node_modules/.bin" binaries do
    if matches!(node_config.package_manager, NodePackageManager::Pnpm) {
        command.env(
            "NODE_PATH",
            node::extend_node_path(path::to_string(
                context
                    .workspace_root
                    .join("node_modules")
                    .join(".pnpm")
                    .join("node_modules"),
            )?),
        );
    }

    Ok(())
}

/// Runs a task command through our toolchain's installed Node.js instance.
/// We accomplish this by executing the Node.js binary as a child process,
/// while passing a file path to a package's node module binary (this is the file
/// being executed). We then also pass arguments defined in the task.
/// This would look something like the following:
///
/// ~/.moon/tools/node/1.2.3/bin/node --inspect /path/to/node_modules/.bin/eslint
///     --cache --color --fix --ext .ts,.tsx,.js,.jsx
#[track_caller]
pub fn create_target_command(
    node: &NodeTool,
    context: &ActionContext,
    project: &Project,
    task: &Task,
    working_dir: &Path,
) -> Result<Command, ToolError> {
    let node_bin = node.get_bin_path()?;
    let node_options = create_node_options(&node.config, context, task)?;
    let mut command = Command::new(node.get_shim_path().unwrap_or(node_bin));

    match task.command.as_str() {
        "node" | "nodejs" => {
            command.args(&node_options);
        }
        "npx" => {
            command = Command::new(node.get_npx_path()?);
        }
        "npm" => {
            command = node.get_npm()?.create_command(node)?;
        }
        "pnpm" => {
            command = node.get_pnpm()?.create_command(node)?;
        }
        "yarn" | "yarnpkg" => {
            command = node.get_yarn()?.create_command(node)?;
        }
        bin => {
            if let Some(new_command) =
                find_package_bin(&mut command, &node_options, &project.root, working_dir, bin)?
            {
                command = new_command;
            }
        }
    };

    command.env("PATH", get_path_env_var(&node.tool.get_install_dir()?));

    prepare_target_command(&mut command, context, task, &node.config)?;

    Ok(command)
}

// This is like the function above, but is for situations where the tool
// has not been configured, and should default to the global "node" found
// in the user's shell.
pub fn create_target_command_without_tool(
    node_config: &NodeConfig,
    context: &ActionContext,
    project: &Project,
    task: &Task,
    working_dir: &Path,
) -> Result<Command, ToolError> {
    let node_options = create_node_options(node_config, context, task)?;
    let mut command = Command::new("node");

    match task.command.as_str() {
        "node" | "nodejs" => {
            command.args(&node_options);
        }
        "npx" | "npm" | "pnpm" | "yarn" | "yarnpkg" => {
            command = Command::new(&task.command);
        }
        bin => {
            if let Some(new_command) =
                find_package_bin(&mut command, &node_options, &project.root, working_dir, bin)?
            {
                command = new_command;
            }
        }
    };

    prepare_target_command(&mut command, context, task, node_config)?;

    Ok(command)
}

pub async fn create_target_hasher(
    node: Option<&NodeTool>,
    project: &Project,
    workspace_root: &Path,
    hasher_config: &HasherConfig,
) -> Result<NodeTargetHasher, ToolError> {
    let mut hasher =
        NodeTargetHasher::new(node.map(|n| n.config.version.clone()).unwrap_or_default());

    let resolved_dependencies =
        if matches!(hasher_config.optimization, HasherOptimization::Accuracy) && node.is_some() {
            node.unwrap()
                .get_package_manager()
                .get_resolved_dependencies(&project.root)
                .await?
        } else {
            FxHashMap::default()
        };

    if let Some(root_package) = PackageJson::read(workspace_root)? {
        hasher.hash_package_json(&root_package, &resolved_dependencies);
    }

    if let Some(package) = PackageJson::read(&project.root)? {
        hasher.hash_package_json(&package, &resolved_dependencies);
    }

    Ok(hasher)
}
