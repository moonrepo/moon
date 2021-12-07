mod errors;
mod helpers;
mod tool;
mod tools;

use dirs::home_dir as get_home_dir;
use monolith_config::constants;
use monolith_config::workspace::{PackageManager as PM, WorkspaceConfig};
use std::fs;
use std::path::{Path, PathBuf};
use tool::{PackageManager, Tool};
use tools::node::NodeTool;
use tools::npm::NpmTool;
use tools::npx::NpxTool;
use tools::pnpm::PnpmTool;
use tools::yarn::YarnTool;

pub use errors::ToolchainError;

fn create_dir(dir: &Path) -> Result<(), ToolchainError> {
    // If path exists but is not a directory, delete it
    if dir.exists() {
        if dir.is_file() && fs::remove_file(dir).is_err() {
            return Err(ToolchainError::FailedToCreateDir);
        }

        // TODO symlink

        // Otherwise attempt to create the directory
    } else if fs::create_dir(dir).is_err() {
        return Err(ToolchainError::FailedToCreateDir);
    }

    Ok(())
}

#[derive(Debug)]
pub struct Toolchain {
    /// The directory where toolchain artifacts are stored.
    /// This is typically ~/.monolith.
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
    pub fn new(config: &WorkspaceConfig) -> Result<Toolchain, ToolchainError> {
        let home_dir = get_home_dir().ok_or(ToolchainError::MissingHomeDir)?;
        let root_dir = home_dir.join(constants::CONFIG_DIRNAME);
        let temp_dir = root_dir.join("temp");
        let tools_dir = root_dir.join("tools");

        create_dir(&root_dir)?;
        create_dir(&temp_dir)?;
        create_dir(&tools_dir)?;

        // Create the instance first, so we can pass to each tool initializer
        let mut toolchain = Toolchain {
            root_dir,
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
        toolchain.npm = Some(NpmTool::new(&toolchain, &config.npm)?);
        toolchain.npx = Some(NpxTool::new(&toolchain));

        if config.node.package_manager.is_some() {
            match config.node.package_manager.as_ref().unwrap() {
                PM::npm => {}
                PM::pnpm => {
                    toolchain.pnpm = Some(PnpmTool::new(&toolchain, &config.pnpm)?);
                }
                PM::yarn => {
                    toolchain.yarn = Some(YarnTool::new(&toolchain, &config.yarn)?);
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

        if !tool.is_installed() {
            tool.install(self).await?;
        }

        Ok(())
    }

    /// Unload the tool by removing any downloaded/installed artifacts.
    /// This can be ran manually, or automatically during a failed load.
    pub async fn unload_tool(&self, tool: &dyn Tool) -> Result<(), ToolchainError> {
        let download_path = tool.get_download_path();

        if tool.is_downloaded() && download_path.is_some() {
            fs::remove_file(download_path.unwrap()).map_err(|_| ToolchainError::FailedToUnload)?;
        }

        if tool.is_installed() {
            fs::remove_dir_all(tool.get_install_dir())
                .map_err(|_| ToolchainError::FailedToUnload)?;
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

    pub fn get_package_manager<T: PackageManager>(&self) -> &dyn PackageManager {
        if self.pnpm.is_some() {
            return self.pnpm.as_ref().unwrap();
        }

        if self.yarn.is_some() {
            return self.yarn.as_ref().unwrap();
        }

        self.npm.as_ref().unwrap()
    }
}
