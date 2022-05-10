use crate::errors::ToolchainError;
use crate::tool::Tool;
use crate::tools::node::NodeTool;
use moon_config::constants::CONFIG_DIRNAME;
use moon_config::WorkspaceConfig;
use moon_logger::{color, debug, trace};
use moon_utils::fs;
use moon_utils::path::get_home_dir;
use std::path::{Path, PathBuf};

async fn create_dir(dir: &Path) -> Result<(), ToolchainError> {
    if dir.exists() {
        if dir.is_file() {
            fs::remove_file(dir).await?;
        }
    } else {
        fs::create_dir_all(dir).await?;
    }

    trace!(target: "moon:toolchain", "Created directory {}", color::path(dir));

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
            target: "moon:toolchain",
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
            node: None,
        })
    }

    pub async fn create(root_dir: &Path) -> Result<Toolchain, ToolchainError> {
        Toolchain::create_from_dir(
            &get_home_dir().ok_or(ToolchainError::MissingHomeDir)?,
            root_dir,
        )
        .await
    }

    /// Download and install all tools into the toolchain.
    /// Return a count of how many tools were installed.
    pub async fn setup(
        &mut self,
        config: &WorkspaceConfig,
        check_versions: bool,
    ) -> Result<u8, ToolchainError> {
        debug!(
            target: "moon:toolchain",
            "Downloading and installing tools",
        );

        let mut installed = 0;
        let mut node = NodeTool::new(&config.node)?;

        if node.load(self, check_versions).await? {
            installed += 1;
        }

        self.node = Some(node);

        Ok(installed)
    }

    /// Uninstall all tools from the toolchain, and delete any temporary files.
    pub async fn teardown(&self) -> Result<(), ToolchainError> {
        debug!(
            target: "moon:toolchain",
            "Tearing down toolchain, uninstalling tools",
        );

        if let Some(node) = &self.node {
            node.unload(self).await?;
        }

        Ok(())
    }

    pub fn get_node(&self) -> &NodeTool {
        self.node.as_ref().unwrap()
    }
}
