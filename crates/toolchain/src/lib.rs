mod errors;
mod helpers;
mod tool;
mod tools;

use dirs::home_dir as get_home_dir;
use errors::ToolchainError;
use monolith_config::constants;
use monolith_config::workspace::{PackageManager as PM, WorkspaceConfig};
use std::fs;
use std::path::{Path, PathBuf};
use tool::PackageManager;
use tools::node::NodeTool;
use tools::npm::NpmTool;
use tools::npx::NpxTool;
use tools::pnpm::PnpmTool;
use tools::yarn::YarnTool;

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
    pub fn load(config: &WorkspaceConfig) -> Result<Toolchain, ToolchainError> {
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
        toolchain.node = Some(NodeTool::load(&toolchain, &config.node)?);
        toolchain.npm = Some(NpmTool::load(&toolchain, &config.npm)?);
        toolchain.npx = Some(NpxTool::load(&toolchain));

        if config.node.package_manager.is_some() {
            match config.node.package_manager.as_ref().unwrap() {
                PM::npm => {}
                PM::pnpm => {
                    toolchain.pnpm = Some(PnpmTool::load(&toolchain, &config.pnpm)?);
                }
                PM::yarn => {
                    toolchain.yarn = Some(YarnTool::load(&toolchain, &config.yarn)?);
                }
            }
        }

        Ok(toolchain)
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

    pub fn get_package_manager<T: PackageManager>(&self) -> Box<&dyn PackageManager> {
        if self.pnpm.is_some() {
            return Box::new(self.pnpm.as_ref().unwrap());
        }

        if self.yarn.is_some() {
            return Box::new(self.yarn.as_ref().unwrap());
        }

        Box::new(self.npm.as_ref().unwrap())
    }
}
