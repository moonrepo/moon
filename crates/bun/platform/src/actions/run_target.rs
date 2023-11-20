use crate::target_hash::BunTargetHash;
use moon_bun_tool::BunTool;
use moon_config::{HasherConfig, HasherOptimization};
use moon_node_lang::{
    node::{self, BinFile},
    PackageJson,
};
use moon_process::Command;
use moon_project::Project;
use moon_task::Task;
use moon_tool::{prepend_path_env_var, DependencyManager, Tool, ToolError};
use moon_utils::path;
use rustc_hash::FxHashMap;
use std::path::Path;

fn find_package_bin(
    command: &mut Command,
    starting_dir: &Path,
    working_dir: &Path,
    bin_name: &str,
) -> miette::Result<Option<Command>> {
    let possible_bin_path = match node::find_package_bin(starting_dir, bin_name)? {
        Some(bin) => bin,
        None => {
            // moon isn't installed as a node module, but probably
            // exists globally, so let's go with that instead of failing
            if bin_name == "moon" {
                return Ok(Some(Command::new(bin_name)));
            }

            return Err(ToolError::MissingBinary("node module".into(), bin_name.to_owned()).into());
        }
    };

    match possible_bin_path {
        // Rust, Go
        BinFile::Binary(bin_path) => {
            return Ok(Some(Command::new(bin_path)));
        }
        // JavaScript
        BinFile::Script(bin_path) => {
            command.arg(path::to_string(
                path::relative_from(bin_path, working_dir).unwrap(),
            )?);
        }
        // Other (Bash)
        BinFile::Other(bin_path, parent_cmd) => {
            let mut cmd = Command::new(parent_cmd);
            cmd.arg(bin_path);

            return Ok(Some(cmd));
        }
    };

    Ok(None)
}

pub fn create_target_command(
    bun: &BunTool,
    project: &Project,
    task: &Task,
    working_dir: &Path,
) -> miette::Result<Command> {
    let mut command = Command::new(bun.get_bin_path()?);

    match task.command.as_str() {
        "bun" | "bunx" => {
            if task.command == "bunx" {
                command.arg("x");
            }
        }
        bin => {
            if let Some(new_command) =
                find_package_bin(&mut command, &project.root, working_dir, bin)?
            {
                command = new_command;
            }
        }
    };

    if !bun.global {
        command.env(
            "PATH",
            prepend_path_env_var([bun.tool.get_exe_path()?.parent().unwrap()]),
        );
    }

    command.args(&task.args).envs(&task.env);

    Ok(command)
}

// This is like the function above, but is for situations where the tool
// has not been configured, and should default to the global "bun" found
// in the user's shell.
pub fn create_target_command_without_tool(
    project: &Project,
    task: &Task,
    working_dir: &Path,
) -> miette::Result<Command> {
    let mut command = Command::new(&task.command);

    if task.command != "bun" && task.command != "bunx" {
        if let Some(new_command) =
            find_package_bin(&mut command, &project.root, working_dir, &task.command)?
        {
            command = new_command;
        }
    }

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
