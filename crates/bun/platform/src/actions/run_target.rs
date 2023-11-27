use crate::target_hash::BunTargetHash;
use moon_bun_tool::BunTool;
use moon_config::{HasherConfig, HasherOptimization};
use moon_node_lang::PackageJson;
use moon_process::Command;
use moon_project::Project;
use moon_task::Task;
use moon_tool::DependencyManager;
use rustc_hash::FxHashMap;
use std::path::Path;

// This is like the function above, but is for situations where the tool
// has not been configured, and should default to the global "bun" found
// in the user's shell.
pub fn create_target_command_without_tool(
    _project: &Project,
    task: &Task,
    _working_dir: &Path,
) -> miette::Result<Command> {
    let mut command = Command::new(&task.command);
    command.args(&task.args).envs(&task.env);

    Ok(command)
}

pub async fn create_target_hasher(
    bun: Option<&BunTool>,
    project: &Project,
    workspace_root: &Path,
    hasher_config: &HasherConfig,
) -> miette::Result<BunTargetHash> {
    let mut hasher = BunTargetHash::new(
        bun.map(|n| n.config.version.as_ref().map(|v| v.to_string()))
            .unwrap_or_default(),
    );

    let resolved_dependencies =
        if matches!(hasher_config.optimization, HasherOptimization::Accuracy) && bun.is_some() {
            bun.unwrap()
                .get_resolved_dependencies(&project.root)
                .await?
        } else {
            FxHashMap::default()
        };

    if let Some(root_package) = PackageJson::read(
        workspace_root.join(bun.map(|n| n.config.packages_root.as_str()).unwrap_or(".")),
    )? {
        hasher.hash_package_json(&root_package, &resolved_dependencies);
    }

    if let Some(package) = PackageJson::read(&project.root)? {
        hasher.hash_package_json(&package, &resolved_dependencies);
    }

    Ok(hasher)
}
