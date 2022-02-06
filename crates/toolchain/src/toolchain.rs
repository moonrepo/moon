use crate::errors::ToolchainError;
use crate::tool::PackageManager;
use crate::tool::Tool;
use crate::tools::node::NodeTool;
use crate::tools::npm::NpmTool;
use crate::tools::pnpm::PnpmTool;
use crate::tools::yarn::YarnTool;
use moon_config::constants::CONFIG_DIRNAME;
use moon_config::{NodeConfig, PackageManager as PM, WorkspaceConfig};
use moon_error::map_io_to_fs_error;
use moon_logger::{color, debug, trace};
use moon_utils::fs::get_home_dir;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

fn create_dir(dir: &Path) -> Result<(), ToolchainError> {
    let handle_error = |e: io::Error| map_io_to_fs_error(e, dir.to_path_buf());

    if dir.exists() {
        if dir.is_file() {
            fs::remove_file(dir).map_err(handle_error)?;
        }
    } else {
        fs::create_dir(dir).map_err(handle_error)?;
    }

    trace!(target: "moon:toolchain", "Created directory {}", color::file_path(dir));

    Ok(())
}

#[derive(Debug)]
pub struct Toolchain {
    /// The directory where toolchain artifacts are stored.
    /// This is typically ~/.moon.
    pub dir: PathBuf,

    /// The directory where temporary files are stored.
    /// This is typically ~/.moon/temp.
    pub temp_dir: PathBuf,

    /// The directory where tools are installed by version.
    /// This is typically ~/.moon/tools.
    pub tools_dir: PathBuf,

    /// The workspace root directory.
    pub workspace_root: PathBuf,

    // Tool instances are private, as we want to lazy load them.
    node: Option<NodeTool>,
    npm: Option<NpmTool>,
    pnpm: Option<PnpmTool>,
    yarn: Option<YarnTool>,
}

impl Toolchain {
    pub fn from(
        config: &WorkspaceConfig,
        base_dir: &Path,
        root_dir: &Path,
    ) -> Result<Toolchain, ToolchainError> {
        let dir = base_dir.join(CONFIG_DIRNAME);
        let temp_dir = dir.join("temp");
        let tools_dir = dir.join("tools");

        debug!(
            target: "moon:toolchain",
            "Creating toolchain at {}",
            color::file_path(&dir)
        );

        create_dir(&dir)?;
        create_dir(&temp_dir)?;
        create_dir(&tools_dir)?;

        // Create the instance first, so we can pass to each tool initializer
        let mut toolchain = Toolchain {
            dir,
            temp_dir,
            tools_dir,
            workspace_root: root_dir.to_path_buf(),
            node: None,
            npm: None,
            pnpm: None,
            yarn: None,
        };

        // Then set the private fields with the tool instances.
        // Order is IMPORTANT here, as some tools rely on others already
        // being instantiated. For example, npm requires node,
        // and pnpm/yarn require npm!
        let node = match &config.node {
            Some(cfg) => cfg.clone(),
            None => NodeConfig::default(),
        };

        toolchain.node = Some(NodeTool::new(&toolchain, &node)?);

        toolchain.npm = Some(NpmTool::new(&toolchain, &node.npm.unwrap_or_default())?);

        if let Some(pm) = node.package_manager {
            match pm {
                PM::Npm => {}
                PM::Pnpm => {
                    toolchain.pnpm =
                        Some(PnpmTool::new(&toolchain, &node.pnpm.unwrap_or_default())?);
                }
                PM::Yarn => {
                    toolchain.yarn =
                        Some(YarnTool::new(&toolchain, &node.yarn.unwrap_or_default())?);
                }
            }
        }

        Ok(toolchain)
    }

    pub fn new(root_dir: &Path, config: &WorkspaceConfig) -> Result<Toolchain, ToolchainError> {
        Toolchain::from(
            config,
            &get_home_dir().ok_or(ToolchainError::MissingHomeDir)?,
            root_dir,
        )
    }

    /// Download and install all tools into the toolchain.
    pub async fn setup(&self) -> Result<(), ToolchainError> {
        debug!(
            target: "moon:toolchain",
            "Setting up toolchain, downloading and installing tools",
        );

        let node = self.get_node();

        self.load_tool(node).await?;

        // Enable corepack when available
        if node.is_corepack_aware() {
            debug!(
                target: "moon:toolchain:node",
                "Enabling corepack for package manager control"
            );

            node.exec_corepack(["enable"]).await?;
        }

        self.load_tool(self.get_npm()).await?;

        if let Some(pnp) = &self.pnpm {
            self.load_tool(pnp).await?;
        }

        if let Some(yarn) = &self.yarn {
            self.load_tool(yarn).await?;
        }

        Ok(())
    }

    /// Uninstall all tools from the toolchain, and delete any temporary files.
    pub async fn teardown(&self) -> Result<(), ToolchainError> {
        debug!(
            target: "moon:toolchain",
            "Tearing down toolchain, uninstalling tools",
        );

        if let Some(yarn) = &self.yarn {
            self.unload_tool(yarn).await?;
        }

        if let Some(pnp) = &self.pnpm {
            self.unload_tool(pnp).await?;
        }

        self.unload_tool(self.get_npm()).await?;
        self.unload_tool(self.get_node()).await?;

        fs::remove_dir_all(&self.dir).map_err(|e| map_io_to_fs_error(e, self.dir.clone()))?;

        Ok(())
    }

    /// Load a tool into the toolchain by downloading an artifact/binary
    /// into the temp folder, then installing it into the tools folder.
    async fn load_tool(&self, tool: &(dyn Tool + Send + Sync)) -> Result<(), ToolchainError> {
        if !tool.is_downloaded() {
            tool.download(None).await?;
        }

        if !tool.is_installed().await? {
            tool.install(self).await?;
        }

        Ok(())
    }

    /// Unload the tool by removing any downloaded/installed artifacts.
    /// This can be ran manually, or automatically during a failed load.
    async fn unload_tool(&self, tool: &(dyn Tool + Send + Sync)) -> Result<(), ToolchainError> {
        if tool.is_downloaded() {
            if let Some(download_path) = tool.get_download_path() {
                fs::remove_file(download_path)
                    .map_err(|e| map_io_to_fs_error(e, download_path.clone()))?;

                trace!(target: "moon:toolchain", "Deleted download {}", color::file_path(download_path));
            }
        }

        if tool.is_installed().await? {
            let install_dir = tool.get_install_dir();

            fs::remove_dir_all(install_dir)
                .map_err(|e| map_io_to_fs_error(e, install_dir.clone()))?;

            trace!(
                target: "moon:toolchain",
                "Deleted installation {}",
                color::file_path(install_dir)
            );
        }

        Ok(())
    }

    pub fn get_node(&self) -> &NodeTool {
        self.node.as_ref().unwrap()
    }

    pub fn get_npm(&self) -> &NpmTool {
        self.npm.as_ref().unwrap()
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

    pub fn get_package_manager(&self) -> &(dyn PackageManager + Send + Sync) {
        if self.pnpm.is_some() {
            return self.get_pnpm().unwrap();
        }

        if self.yarn.is_some() {
            return self.get_yarn().unwrap();
        }

        self.get_npm()
    }
}
