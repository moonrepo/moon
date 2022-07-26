use crate::context::{ActionContext, ProfileType};
use crate::errors::ActionError;
use moon_config::PackageManager;
use moon_error::MoonError;
use moon_lang_node::{node, package::PackageJson, tsconfig::TsConfigJson};
use moon_logger::{color, trace};
use moon_plugin_node::NodeTargetHasher;
use moon_project::Project;
use moon_task::Task;
use moon_toolchain::{get_path_env_var, Executable};
use moon_utils::process::Command;
use moon_utils::{path, string_vec};
use moon_workspace::Workspace;

fn create_node_options(
    context: &ActionContext,
    workspace: &Workspace,
    task: &Task,
) -> Result<Vec<String>, MoonError> {
    let mut options = string_vec![
        // "--inspect", // Enable node inspector
        "--preserve-symlinks",
        "--title",
        &task.target,
        "--unhandled-rejections",
        "throw",
    ];

    if let Some(profile) = &context.profile {
        let prof_dir = workspace.cache.get_target_dir(&task.target);

        match profile {
            ProfileType::Cpu => {
                trace!(
                    target: "moon:action:run-node-target",
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
                    target: "moon:action:run-node-target",
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
) -> Result<Command, ActionError> {
    let toolchain = &workspace.toolchain;
    let node = toolchain.get_node();
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
            cmd = node.get_pnpm().unwrap().get_bin_path().clone();
        }
        "yarn" => {
            cmd = node.get_yarn().unwrap().get_bin_path().clone();
        }
        bin => {
            let bin_path = path::relative_from(
                node.get_package_manager()
                    .find_package_bin(toolchain, &project.root, bin)
                    .await?,
                &project.root,
            )
            .unwrap();

            if bin_path.extension().unwrap_or_default() == "exe" {
                cmd = bin_path;
            } else {
                args.extend(create_node_options(context, workspace, task)?);
                args.push(path::to_string(&bin_path)?);
            }
        }
    };

    // Create the command
    let mut command = Command::new(cmd);

    command.args(&args).args(&task.args).envs(&task.env).env(
        "PATH",
        get_path_env_var(node.get_bin_path().parent().unwrap()),
    );

    // This functionality mimics what pnpm's "node_modules/.bin" binaries do
    if matches!(node.config.package_manager, PackageManager::Pnpm) {
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

pub fn create_target_hasher(
    workspace: &Workspace,
    project: &Project,
) -> Result<NodeTargetHasher, ActionError> {
    let mut hasher = NodeTargetHasher::new(workspace.config.node.version.clone());

    // Hash root configs first
    if let Some(root_package) = PackageJson::read(&workspace.root)? {
        hasher.hash_package_json(&root_package);
    }

    if let Some(root_tsconfig) = TsConfigJson::read_with_name(
        &workspace.root,
        &workspace.config.typescript.root_config_file_name,
    )? {
        hasher.hash_tsconfig_json(&root_tsconfig);
    }

    // Hash project configs second so they can override
    if let Some(package) = PackageJson::read(&project.root)? {
        hasher.hash_package_json(&package);
    }

    if let Some(tsconfig) = TsConfigJson::read_with_name(
        &project.root,
        &workspace.config.typescript.project_config_file_name,
    )? {
        hasher.hash_tsconfig_json(&tsconfig);
    }

    Ok(hasher)
}
