mod errors;
mod helpers;
mod tool;
mod tools;

use dirs::home_dir as get_home_dir;
use monolith_config::constants;
use monolith_config::workspace::{
    NpmConfig, PackageManager as PM, PnpmConfig, WorkspaceConfig, YarnConfig,
};
use std::fs;
use std::path::{Path, PathBuf};
use tool::PackageManager;
use tools::node::NodeTool;
use tools::npm::NpmTool;
use tools::npx::NpxTool;
use tools::pnpm::PnpmTool;
use tools::yarn::YarnTool;

pub use errors::ToolchainError;
pub use tool::Tool;

fn create_dir(dir: &Path) -> Result<(), ToolchainError> {
    if dir.exists() {
        if dir.is_file() {
            fs::remove_file(dir)?;
        }
    } else {
        fs::create_dir(dir)?;
    }

    Ok(())
}

#[derive(Debug)]
pub struct Toolchain {
    /// The directory where toolchain artifacts are stored.
    /// This is typically ~/.monolith.
    pub home_dir: PathBuf,

    /// The workspace root directory.
    pub root_dir: PathBuf,

    /// The directory where temporary files are stored.
    /// This is typically ~/.monolith/temp.
    pub temp_dir: PathBuf,

    /// The directory where tools are installed by version.
    /// This is typically ~/.monolith/tools.
    pub tools_dir: PathBuf,

    // Tool instances are private, as we want to lazy load them.
    node: Option<NodeTool>,
    npm: Option<NpmTool>,
    npx: Option<NpxTool>,
    pnpm: Option<PnpmTool>,
    yarn: Option<YarnTool>,
}

impl Toolchain {
    pub fn new(config: &WorkspaceConfig, root_dir: &Path) -> Result<Toolchain, ToolchainError> {
        let user_home_dir = get_home_dir().ok_or(ToolchainError::MissingHomeDir)?;
        let home_dir = user_home_dir.join(constants::CONFIG_DIRNAME);
        let temp_dir = home_dir.join("temp");
        let tools_dir = home_dir.join("tools");

        create_dir(&home_dir)?;
        create_dir(&temp_dir)?;
        create_dir(&tools_dir)?;

        // Create the instance first, so we can pass to each tool initializer
        let mut toolchain = Toolchain {
            home_dir,
            root_dir: root_dir.to_path_buf(),
            temp_dir,
            tools_dir,
            node: None,
            npm: None,
            npx: None,
            pnpm: None,
            yarn: None,
        };

        // Then set the private fields with the tool instances.
        // Order is IMPORTANT here, as some tools rely on others already
        // being instantiated. For example, npm requires node,
        // and pnpm/yarn require npm!
        toolchain.node = Some(NodeTool::new(&toolchain, &config.node)?);

        toolchain.npm = Some(NpmTool::new(
            &toolchain,
            config.node.npm.as_ref().unwrap_or(&NpmConfig::default()), // TODO: Better way?
        )?);

        toolchain.npx = Some(NpxTool::new(&toolchain));

        if config.node.package_manager.is_some() {
            match config.node.package_manager.as_ref().unwrap() {
                PM::npm => {}
                PM::pnpm => {
                    toolchain.pnpm = Some(PnpmTool::new(
                        &toolchain,
                        config.node.pnpm.as_ref().unwrap_or(&PnpmConfig::default()),
                    )?);
                }
                PM::yarn => {
                    toolchain.yarn = Some(YarnTool::new(
                        &toolchain,
                        config.node.yarn.as_ref().unwrap_or(&YarnConfig::default()),
                    )?);
                }
            }
        }

        Ok(toolchain)
    }

    /// Load a tool into the toolchain by downloading an artifact/binary
    /// into the temp folder, then installing it into the tools folder.
    pub async fn load_tool(&self, tool: &dyn Tool) -> Result<(), ToolchainError> {
        if !tool.is_downloaded() {
            tool.download().await?;
        }

        if !tool.is_installed().await? {
            tool.install(self).await?;
        }

        Ok(())
    }

    /// Unload the tool by removing any downloaded/installed artifacts.
    /// This can be ran manually, or automatically during a failed load.
    pub async fn unload_tool(&self, tool: &dyn Tool) -> Result<(), ToolchainError> {
        let download_path = tool.get_download_path();

        if download_path.is_some() && tool.is_downloaded() {
            fs::remove_file(download_path.unwrap())?;
        }

        if tool.is_installed().await? {
            fs::remove_dir_all(tool.get_install_dir())?;
        }

        Ok(())
    }

    pub fn get_node(&self) -> &NodeTool {
        self.node.as_ref().unwrap()
    }

    pub fn get_npm(&self) -> &NpmTool {
        self.npm.as_ref().unwrap()
    }

    pub fn get_npx(&self) -> &NpxTool {
        self.npx.as_ref().unwrap()
    }

    pub fn get_pnpm(&self) -> Option<&PnpmTool> {
        match &self.pnpm {
            Some(tool) => Some(tool),
            None => None,
        }
    }

    pub fn get_yarn(&self) -> Option<&YarnTool> {
        match &self.yarn {
            Some(tool) => Some(tool),
            None => None,
        }
    }

    pub fn get_package_manager(&self) -> &dyn PackageManager {
        if self.pnpm.is_some() {
            return self.get_pnpm().unwrap();
        }

        if self.yarn.is_some() {
            return self.get_yarn().unwrap();
        }

        self.get_npm()
    }
}
