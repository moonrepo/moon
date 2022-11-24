use crate::errors::ToolchainError;
use crate::helpers::{download_file_from_url, unpack};
use crate::tools::node::NodeTool;
use crate::traits::{DependencyManager, Executable, Installable, Lifecycle};
use crate::{get_path_env_var, ToolchainPaths};
use async_trait::async_trait;
use moon_config::YarnConfig;
use moon_lang::LockfileDependencyVersions;
use moon_logger::{color, debug, Logable};
use moon_node_lang::{node, yarn, YARN};
use moon_utils::process::Command;
use moon_utils::{fs, get_workspace_root, is_ci, path};
use rustc_hash::FxHashMap;
use std::env;
use std::path::{Path, PathBuf};

#[async_trait]
impl Lifecycle<NodeTool> for YarnTool {
    async fn setup(&mut self, node: &NodeTool, check_version: bool) -> Result<u8, ToolchainError> {
        if !check_version || self.is_v1() {
            return Ok(0);
        }

        // We also dont want to *always* run this, so only run it when
        // we detect different yarn version files in the repo. Also note, we don't
        // have access to the workspace root here...
        let root = match env::var("MOON_WORKSPACE_ROOT") {
            Ok(root) => PathBuf::from(root),
            Err(_) => env::current_dir().unwrap_or_default(),
        };

        let yarn_bin = root
            .join(".yarn/releases")
            .join(format!("yarn-{}.cjs", self.config.version));

        if !yarn_bin.exists() {
            // We must do this here instead of `install`, because the bin path
            // isn't available yet during installation, only after!
            debug!(
                target: self.get_log_target(),
                "Updating package manager version with {}",
                color::shell(format!("yarn set version {}", self.config.version))
            );

            self.create_command(node)
                .args(["set", "version", &self.config.version])
                .exec_capture_output()
                .await?;

            if let Some(plugins) = &self.config.plugins {
                for plugin in plugins {
                    self.create_command(node)
                        .args(["plugin", "import", plugin])
                        .exec_capture_output()
                        .await?;
                }
            }
        }

        Ok(1)
    }
}
