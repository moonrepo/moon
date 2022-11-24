use crate::tools::npm::NpmTool;
use crate::tools::pnpm::PnpmTool;
use crate::tools::yarn::YarnTool;
use crate::{errors::ToolchainError, DependencyManager, RuntimeTool};
use async_trait::async_trait;
use moon_config::{NodeConfig, NodePackageManager};
use probe_core::{Probe, Resolvable, Tool};
use probe_node::NodeLanguage;

#[derive(Debug)]
pub struct NodeTool {
    pub config: NodeConfig,

    pub tool: NodeLanguage,

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
    fn get_version(&self) -> &str {
        self.tool.get_resolved_version()
    }

    async fn setup(&mut self) -> Result<u8, ToolchainError> {
        let mut count = 0;

        if !self.tool.is_setup()? && self.tool.setup(&self.config.version).await? {
            count += 1;
        }

        Ok(count)
    }

    async fn teardown(&mut self) -> Result<(), ToolchainError> {
        self.tool.teardown().await?;

        Ok(())
    }
}
