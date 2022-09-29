use crate::errors::ToolchainError;
use crate::helpers::LOG_TARGET;
use crate::manager::ToolManager;
use crate::tools::node::NodeTool;
use moon_constants::CONFIG_DIRNAME;
use moon_logger::{color, debug, trace};
use moon_utils::{fs, path};
use std::path::{Path, PathBuf};

async fn create_dir(dir: &Path) -> Result<(), ToolchainError> {
    trace!(
        target: LOG_TARGET,
        "Creating directory {}",
        color::path(dir)
    );

    if dir.exists() {
        if dir.is_file() {
            fs::remove_file(dir).await?;
        }
    } else {
        fs::create_dir_all(dir).await?;
    }

    Ok(())
}

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

    /// Node.js!
    pub node: ToolManager<NodeTool>,
}

impl Toolchain {
    pub async fn create_from_dir(
        base_dir: &Path,
        root_dir: &Path,
    ) -> Result<Toolchain, ToolchainError> {
        let dir = base_dir.join(CONFIG_DIRNAME);
        let temp_dir = dir.join("temp");
        let tools_dir = dir.join("tools");

        debug!(
            target: LOG_TARGET,
            "Creating toolchain at {}",
            color::path(&dir)
        );

        create_dir(&dir).await?;
        create_dir(&temp_dir).await?;
        create_dir(&tools_dir).await?;

        Ok(Toolchain {
            dir,
            temp_dir,
            tools_dir,
            workspace_root: root_dir.to_path_buf(),
            // Tools
            node: ToolManager::new("todo"),
        })
    }

    pub async fn create(root_dir: &Path) -> Result<Toolchain, ToolchainError> {
        Toolchain::create_from_dir(
            &path::get_home_dir().ok_or(ToolchainError::MissingHomeDir)?,
            root_dir,
        )
        .await
    }

    /// Uninstall all tools from the toolchain, and delete any temporary files.
    pub async fn teardown(&mut self) -> Result<(), ToolchainError> {
        debug!(
            target: LOG_TARGET,
            "Tearing down toolchain, uninstalling tools",
        );

        self.node.teardown().await?;

        Ok(())
    }
}
