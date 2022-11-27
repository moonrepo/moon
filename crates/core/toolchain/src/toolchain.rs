use crate::errors::ToolchainError;
use crate::manager::ToolManager;
use crate::tools::node::NodeTool;
use moon_config::WorkspaceConfig;
use moon_constants::CONFIG_DIRNAME;
use moon_logger::{color, debug};
use moon_platform::{Runtime, Version};
use moon_utils::{fs, path};
use proto_core::Probe;
use std::path::{Path, PathBuf};

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
            target: "moon:toolchain",
            "Creating toolchain at {}",
            color::path(&dir)
        );

        fs::create_dir_all(&dir).await?;

        let mut toolchain = Toolchain {
            dir,
            // Tools
            node: ToolManager::new(Runtime::Node(Version::default())),
        };
        let proto = toolchain.get_paths();

        if let Some(node_config) = &workspace_config.node {
            toolchain
                .node
                .register(NodeTool::new(&proto, node_config)?, true);
        }

        Ok(toolchain)
    }

    pub fn get_paths(&self) -> Probe {
        Probe::new(&self.dir)
    }

    /// Uninstall all tools from the toolchain, and delete any temporary files.
    pub async fn teardown(&mut self) -> Result<(), ToolchainError> {
        debug!(
            target: "moon:toolchain",
            "Tearing down toolchain, uninstalling tools",
        );

        self.node.teardown_all().await?;

        Ok(())
    }
}
