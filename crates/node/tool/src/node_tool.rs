use crate::npm_tool::NpmTool;
use crate::pnpm_tool::PnpmTool;
use crate::yarn_tool::YarnTool;
use moon_config::{NodeConfig, NodePackageManager};
use moon_logger::debug;
use moon_node_lang::node;
use moon_terminal::{print_checkpoint, Checkpoint};
use moon_tool::{get_path_env_var, DependencyManager, Tool, ToolError};
use moon_utils::process::Command;
use proto::{
    async_trait, node::NodeLanguage, Describable, Executable, Installable, Proto, Resolvable,
    Shimable, Tool as ProtoTool,
};
use rustc_hash::FxHashMap;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct NodeTool {
    pub config: NodeConfig,

    pub tool: NodeLanguage,

    npm: Option<NpmTool>,

    pnpm: Option<PnpmTool>,

    yarn: Option<YarnTool>,
}

impl NodeTool {
    pub fn new(proto: &Proto, config: &NodeConfig, version: &str) -> Result<NodeTool, ToolError> {
        let mut node = NodeTool {
            config: config.to_owned(),
            tool: NodeLanguage::new(proto),
            npm: None,
            pnpm: None,
            yarn: None,
        };

        node.config.version = Some(version.to_owned());

        match config.package_manager {
            NodePackageManager::Npm => {
                node.npm = Some(NpmTool::new(proto, &config.npm)?);
            }
            NodePackageManager::Pnpm => {
                node.pnpm = Some(PnpmTool::new(proto, &config.pnpm)?);
            }
            NodePackageManager::Yarn => {
                node.yarn = Some(YarnTool::new(proto, &config.yarn)?);
            }
        };

        Ok(node)
    }

    pub async fn exec_package(
        &self,
        package: &str,
        args: &[&str],
        working_dir: &Path,
    ) -> Result<(), ToolError> {
        let mut exec_args = vec!["--silent", "--package", package, "--"];
        exec_args.extend(args);

        Command::new(self.get_npx_path()?)
            .args(exec_args)
            .cwd(working_dir)
            .env("PATH", get_path_env_var(&self.tool.get_install_dir()?))
            .exec_stream_output()
            .await?;

        Ok(())
    }

    /// Return the `npm` package manager.
    pub fn get_npm(&self) -> Result<&NpmTool, ToolError> {
        match &self.npm {
            Some(npm) => Ok(npm),
            None => Err(ToolError::UnknownTool("npm".into())),
        }
    }

    pub fn get_npx_path(&self) -> Result<PathBuf, ToolError> {
        Ok(node::find_package_manager_bin(
            self.tool.get_install_dir()?,
            "npx",
        ))
    }

    /// Return the `pnpm` package manager.
    pub fn get_pnpm(&self) -> Result<&PnpmTool, ToolError> {
        match &self.pnpm {
            Some(pnpm) => Ok(pnpm),
            None => Err(ToolError::UnknownTool("pnpm".into())),
        }
    }

    /// Return the `yarn` package manager.
    pub fn get_yarn(&self) -> Result<&YarnTool, ToolError> {
        match &self.yarn {
            Some(yarn) => Ok(yarn),
            None => Err(ToolError::UnknownTool("yarn".into())),
        }
    }

    pub fn get_package_manager(&self) -> &(dyn DependencyManager<Self> + Send + Sync) {
        if self.pnpm.is_some() {
            return self.get_pnpm().unwrap();
        }

        if self.yarn.is_some() {
            return self.get_yarn().unwrap();
        }

        if self.npm.is_some() {
            return self.get_npm().unwrap();
        }

        panic!("No package manager, how's this possible?");
    }
}

#[async_trait]
impl Tool for NodeTool {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn get_bin_path(&self) -> Result<&Path, ToolError> {
        Ok(self.tool.get_bin_path()?)
    }

    fn get_shim_path(&self) -> Option<&Path> {
        self.tool.get_shim_path()
    }

    fn get_version(&self) -> &str {
        self.tool.get_resolved_version()
    }

    async fn setup(
        &mut self,
        last_versions: &mut FxHashMap<String, String>,
    ) -> Result<u8, ToolError> {
        let mut installed = 0;
        let version_clone = self.config.version.clone();

        let Some(version) = version_clone else {
            return Ok(installed);
        };

        if self.tool.is_setup(&version).await? {
            debug!(target: self.tool.get_log_target(), "Node.js has already been setup");
        } else {
            let setup = match last_versions.get("node") {
                Some(last) => &version != last,
                None => true,
            };

            if setup || !self.tool.get_install_dir()?.exists() {
                print_checkpoint(format!("installing node v{version}"), Checkpoint::Setup);

                if self.tool.setup(&version).await? {
                    last_versions.insert("node".into(), version.to_string());
                    installed += 1;
                }
            }
        }

        if let Some(npm) = &mut self.npm {
            installed += npm.setup(last_versions).await?;
        }

        if let Some(pnpm) = &mut self.pnpm {
            installed += pnpm.setup(last_versions).await?;
        }

        if self.yarn.is_some() {
            let mut yarn = self.yarn.take().unwrap();

            installed += yarn.setup(last_versions).await?;
            yarn.set_version(self).await?;

            self.yarn = Some(yarn);
        }

        Ok(installed)
    }

    async fn teardown(&mut self) -> Result<(), ToolError> {
        self.tool.teardown().await?;

        Ok(())
    }
}
