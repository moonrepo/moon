use crate::get_path_env_var;
use crate::tools::npm::NpmTool;
use crate::tools::pnpm::PnpmTool;
use crate::tools::yarn::YarnTool;
use crate::{errors::ToolchainError, DependencyManager, RuntimeTool};
use moon_config::{NodeConfig, NodePackageManager};
use moon_node_lang::node;
use moon_utils::process::Command;
use probe_core::{async_trait, Executable, Installable, Probe, Resolvable, Tool};
use probe_node::NodeLanguage;
use rustc_hash::FxHashMap;
use std::path::Path;

#[derive(Debug)]
pub struct NodeTool {
    pub config: NodeConfig,

    tool: NodeLanguage,

    npm: Option<NpmTool>,

    pnpm: Option<PnpmTool>,

    yarn: Option<YarnTool>,
}

impl NodeTool {
    pub fn new(probe: &Probe, config: &NodeConfig) -> Result<NodeTool, ToolchainError> {
        let mut node = NodeTool {
            config: config.to_owned(),
            tool: NodeLanguage::new(probe, Some(&config.version)),
            npm: None,
            pnpm: None,
            yarn: None,
        };

        match config.package_manager {
            NodePackageManager::Npm => {
                node.npm = Some(NpmTool::new(probe, &config.npm)?);
            }
            NodePackageManager::Pnpm => {
                node.pnpm = Some(PnpmTool::new(probe, &config.pnpm)?);
            }
            NodePackageManager::Yarn => {
                node.yarn = Some(YarnTool::new(probe, &config.yarn)?);
            }
        };

        Ok(node)
    }

    pub async fn exec_package(
        &self,
        package: &str,
        args: &[&str],
        working_dir: &Path,
    ) -> Result<(), ToolchainError> {
        let mut exec_args = vec!["--silent", "--package", package, "--"];
        let install_dir = self.tool.get_install_dir()?;

        exec_args.extend(args);

        let npx_path = node::find_package_manager_bin(&install_dir, "npx");

        Command::new(&npx_path)
            .args(exec_args)
            .cwd(working_dir)
            .env("PATH", get_path_env_var(&install_dir))
            .exec_stream_output()
            .await?;

        Ok(())
    }

    pub fn find_package_bin(
        &self,
        starting_dir: &Path,
        bin_name: &str,
    ) -> Result<node::BinFile, ToolchainError> {
        match node::find_package_bin(starting_dir, bin_name)? {
            Some(bin) => Ok(bin),
            None => Err(ToolchainError::MissingNodeModuleBin(bin_name.to_owned())),
        }
    }

    /// Return the `npm` package manager.
    pub fn get_npm(&self) -> Result<&NpmTool, ToolchainError> {
        match &self.npm {
            Some(npm) => Ok(npm),
            None => Err(ToolchainError::MissingTool("npm".into())),
        }
    }

    /// Return the `pnpm` package manager.
    pub fn get_pnpm(&self) -> Result<&PnpmTool, ToolchainError> {
        match &self.pnpm {
            Some(pnpm) => Ok(pnpm),
            None => Err(ToolchainError::MissingTool("pnpm".into())),
        }
    }

    /// Return the `yarn` package manager.
    pub fn get_yarn(&self) -> Result<&YarnTool, ToolchainError> {
        match &self.yarn {
            Some(yarn) => Ok(yarn),
            None => Err(ToolchainError::MissingTool("yarn".into())),
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
impl RuntimeTool for NodeTool {
    fn get_bin_path(&self) -> Result<&Path, ToolchainError> {
        Ok(self.tool.get_bin_path()?)
    }

    fn get_version(&self) -> &str {
        self.tool.get_resolved_version()
    }

    async fn setup(
        &mut self,
        last_versions: &mut FxHashMap<String, String>,
    ) -> Result<u8, ToolchainError> {
        let mut installed = 0;

        if !self.tool.is_setup()? {
            let setup = match last_versions.get("node") {
                Some(last) => &self.config.version != last,
                None => true,
            };

            if setup && self.tool.setup(&self.config.version).await? {
                last_versions.insert("node".into(), self.get_version().to_owned());
                installed += 1;
            }
        }

        if let Some(npm) = &mut self.npm {
            installed += npm.setup(last_versions).await?;
        }

        if let Some(pnpm) = &mut self.pnpm {
            installed += pnpm.setup(last_versions).await?;
        }

        if let Some(yarn) = &mut self.yarn {
            installed += yarn.setup(last_versions).await?;
        }

        Ok(installed)
    }

    async fn teardown(&mut self) -> Result<(), ToolchainError> {
        self.tool.teardown().await?;

        Ok(())
    }
}
