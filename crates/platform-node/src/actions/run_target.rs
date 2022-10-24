use crate::hasher::NodeTargetHasher;
use moon_action::{ActionContext, ProfileType};
use moon_config::{HasherOptimization, NodePackageManager};
use moon_error::MoonError;
use moon_lang_node::{
    node::{self, BinFile},
    package::PackageJson,
    tsconfig::TsConfigJson,
};
use moon_logger::{color, trace};
use moon_project::Project;
use moon_task::Task;
use moon_toolchain::{get_path_env_var, Executable};
use moon_utils::process::Command;
use moon_utils::{path, string_vec};
use moon_workspace::{Workspace, WorkspaceError};
use std::collections::HashMap;

const LOG_TARGET: &str = "moon:platform-node:run-target";

fn create_node_options(
    context: &ActionContext,
    workspace: &Workspace,
    task: &Task,
) -> Result<Vec<String>, MoonError> {
    let mut options = string_vec![
        // "--inspect", // Enable node inspector
        "--title",
        &task.target,
    ];

    if let Some(node_config) = &workspace.config.node {
        if !node_config.bin_run_args.is_empty() {
            options.extend(node_config.bin_run_args.to_owned());
        }
    }

    if let Some(profile) = &context.profile {
        let prof_dir = workspace.cache.get_target_dir(&task.target);

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

/// Runs a task command through our toolchain's installed Node.js instance.
/// We accomplish this by executing the Node.js binary as a child process,
/// while passing a file path to a package's node module binary (this is the file
/// being executed). We then also pass arguments defined in the task.
/// This would look something like the following:
///
/// ~/.moon/tools/node/1.2.3/bin/node --inspect /path/to/node_modules/.bin/eslint
///     --cache --color --fix --ext .ts,.tsx,.js,.jsx
#[track_caller]
pub async fn create_target_command(
    context: &ActionContext,
    workspace: &Workspace,
    project: &Project,
    task: &Task,
) -> Result<Command, WorkspaceError> {
    let mut node = workspace.toolchain.node.get()?;

    // If a version override exists, use it for the cmmand
    if let Some(node_config) = &project.config.workspace.node {
        if let Some(version_override) = &node_config.version {
            node = workspace.toolchain.node.get_for_version(version_override)?;
        }
    }

    let mut cmd = node.get_bin_path().clone();
    let mut args = vec![];

    match task.command.as_str() {
        "node" => {
            args.extend(create_node_options(context, workspace, task)?);
        }
        "npm" => {
            cmd = node.get_npm().get_bin_path().clone();
        }
        "pnpm" => {
            cmd = node
                .get_pnpm()
                .expect("pnpm must be enabled")
                .get_bin_path()
                .clone();
        }
        "yarn" => {
            cmd = node
                .get_yarn()
                .expect("yarn must be enabled")
                .get_bin_path()
                .clone();
        }
        bin => {
            match node.find_package_bin(&project.root, bin)? {
                // Rust, Go
                BinFile::Binary(bin_path) => {
                    cmd = bin_path;
                }
                // JavaScript
                BinFile::Script(bin_path) => {
                    args.extend(create_node_options(context, workspace, task)?);
                    args.push(path::to_string(
                        path::relative_from(bin_path, &project.root).unwrap(),
                    )?);
                }
            };
        }
    };

    // Create the command
    let mut command = Command::new(cmd);

    command.args(&args).args(&task.args).envs(&task.env).env(
        "PATH",
        get_path_env_var(node.get_bin_path().parent().unwrap()),
    );

    // This functionality mimics what pnpm's "node_modules/.bin" binaries do
    if matches!(node.config.package_manager, NodePackageManager::Pnpm) {
        command.env(
            "NODE_PATH",
            node::extend_node_path(path::to_string(
                workspace
                    .root
                    .join("node_modules")
                    .join(".pnpm")
                    .join("node_modules"),
            )?),
        );
    }

    Ok(command)
}

pub async fn create_target_hasher(
    workspace: &Workspace,
    project: &Project,
) -> Result<NodeTargetHasher, WorkspaceError> {
    let node = workspace.toolchain.node.get()?;
    let mut hasher = NodeTargetHasher::new(node.config.version.clone());

    let resolved_dependencies = if matches!(
        workspace.config.hasher.optimization,
        HasherOptimization::Accuracy
    ) {
        node.get_package_manager()
            .get_resolved_depenencies(&project.root)
            .await?
    } else {
        HashMap::new()
    };

    if let Some(root_package) = PackageJson::read(&workspace.root)? {
        hasher.hash_package_json(&root_package, &resolved_dependencies);
    }

    if let Some(package) = PackageJson::read(&project.root)? {
        hasher.hash_package_json(&package, &resolved_dependencies);
    }

    if let Some(typescript_config) = &workspace.config.typescript {
        if let Some(root_tsconfig) =
            TsConfigJson::read_with_name(&workspace.root, &typescript_config.root_config_file_name)?
        {
            hasher.hash_tsconfig_json(&root_tsconfig);
        }

        if let Some(tsconfig) = TsConfigJson::read_with_name(
            &project.root,
            &typescript_config.project_config_file_name,
        )? {
            hasher.hash_tsconfig_json(&tsconfig);
        }
    }

    Ok(hasher)
}
