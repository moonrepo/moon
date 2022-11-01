use crate::errors::ToolchainError;
use crate::helpers::LOG_TARGET;
use crate::manager::ToolManager;
use crate::tools::node::NodeTool;
use moon_config::WorkspaceConfig;
use moon_constants::CONFIG_DIRNAME;
use moon_logger::{color, debug, trace};
use moon_platform::Runtime;
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

pub struct ToolchainPaths {
    pub temp: PathBuf,
    pub tools: PathBuf,
}

#[derive(Debug)]
pub struct Toolchain {
    /// The directory where toolchain artifacts are stored.
    /// This is typically ~/.moon.
    pub dir: PathBuf,

    /// Node.js!
    pub node: ToolManager<NodeTool>,
}

impl Toolchain {
    pub async fn load(workspace_config: &WorkspaceConfig) -> Result<Toolchain, ToolchainError> {
        Toolchain::load_from(
            path::get_home_dir().ok_or(ToolchainError::MissingHomeDir)?,
            workspace_config,
        )
        .await
    }

    pub async fn load_from<P: AsRef<Path>>(
        base_dir: P,
        workspace_config: &WorkspaceConfig,
    ) -> Result<Toolchain, ToolchainError> {
        let dir = base_dir.as_ref().join(CONFIG_DIRNAME);

        debug!(
            target: LOG_TARGET,
            "Creating toolchain at {}",
            color::path(&dir)
        );

        create_dir(&dir).await?;

        let mut toolchain = Toolchain {
            dir,
            // Tools
            node: ToolManager::new(Runtime::Node("latest".into())),
        };

        let paths = toolchain.get_paths();

        if let Some(node_config) = &workspace_config.node {
            toolchain
                .node
                .register(NodeTool::new(&paths, node_config)?, true);
        }

        Ok(toolchain)
    }

    pub fn get_paths(&self) -> ToolchainPaths {
        ToolchainPaths {
            /// The directory where temporary files are stored.
            /// This is typically ~/.moon/temp.
            temp: self.dir.join("temp"),
            /// The directory where tools are installed by version.
            /// This is typically ~/.moon/tools.
            tools: self.dir.join("tools"),
        }
    }

    /// Uninstall all tools from the toolchain, and delete any temporary files.
    pub async fn teardown(&mut self) -> Result<(), ToolchainError> {
        debug!(
            target: LOG_TARGET,
            "Tearing down toolchain, uninstalling tools",
        );

        self.node.teardown_all().await?;

        Ok(())
    }
}
