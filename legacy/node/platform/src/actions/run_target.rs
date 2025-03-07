use crate::target_hash::NodeTargetHash;
use moon_action_context::{ActionContext, ProfileType};
use moon_config::{HasherConfig, HasherOptimization, NodeConfig, NodePackageManager};
use moon_logger::trace;
use moon_node_lang::{PackageJsonCache, node};
use moon_node_tool::NodeTool;
use moon_process::Command;
use moon_project::Project;
use moon_task::Task;
use moon_utils::{get_cache_dir, path, string_vec};
use rustc_hash::FxHashMap;
use starbase_styles::color;
use std::path::Path;

const LOG_TARGET: &str = "moon:node-platform:run-task";

fn create_node_options(
    node_config: &NodeConfig,
    context: &ActionContext,
    task: &Task,
) -> miette::Result<Vec<String>> {
    let mut options = string_vec![
        // "--inspect", // Enable node inspector
        // "--title",
        // &task.target.id,
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
                    color::label(&task.target),
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
                    color::label(&task.target),
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

fn prepare_target_command(
    command: &mut Command,
    task: &Task,
    node_config: &NodeConfig,
    workspace_root: &Path,
) -> miette::Result<()> {
    command.args(&task.args).envs(&task.env);

    // This functionality mimics what pnpm's "node_modules/.bin" binaries do
    if matches!(node_config.package_manager, NodePackageManager::Pnpm) {
        command.env(
            "NODE_PATH",
            node::extend_node_path(path::to_string(
                workspace_root
                    .join(&node_config.packages_root)
                    .join("node_modules")
                    .join(".pnpm")
                    .join("node_modules"),
            )?),
        );
    }

    Ok(())
}

pub fn create_target_command_without_tool(
    node_config: &NodeConfig,
    context: &ActionContext,
    _project: &Project,
    task: &Task,
    workspace_root: &Path,
) -> miette::Result<Command> {
    let mut command = Command::new("node");

    match task.command.as_str() {
        "node" | "nodejs" => {
            command.args(create_node_options(node_config, context, task)?);
        }
        "npx" | "npm" | "pnpm" | "pnpx" | "yarn" | "yarnpkg" | "bun" | "bunx" => {
            command = Command::new(&task.command);
        }
        bin => {
            command = Command::new(bin);
        }
    };

    prepare_target_command(&mut command, task, node_config, workspace_root)?;

    Ok(command)
}

pub async fn create_target_hasher(
    node: Option<&NodeTool>,
    project: &Project,
    workspace_root: &Path,
    hasher_config: &HasherConfig,
) -> miette::Result<NodeTargetHash> {
    let mut hasher = NodeTargetHash::new(
        node.map(|n| n.config.version.as_ref().map(|v| v.to_string()))
            .unwrap_or_default(),
    );

    let resolved_dependencies =
        if matches!(hasher_config.optimization, HasherOptimization::Accuracy) && node.is_some() {
            node.unwrap()
                .get_package_manager()
                .get_resolved_dependencies(&project.root)
                .await?
        } else {
            FxHashMap::default()
        };

    if let Some(root_package) = PackageJsonCache::read(
        workspace_root.join(node.map(|n| n.config.packages_root.as_str()).unwrap_or(".")),
    )? {
        hasher.hash_package_json(&root_package.data, &resolved_dependencies);
    }

    if let Some(package) = PackageJsonCache::read(&project.root)? {
        hasher.hash_package_json(&package.data, &resolved_dependencies);
    }

    Ok(hasher)
}
