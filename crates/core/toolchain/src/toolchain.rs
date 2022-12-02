use crate::errors::ToolchainError;
use crate::manager::ToolManager;
use crate::tools::node::NodeTool;
use moon_config::ToolchainConfig;
use moon_constants::CONFIG_DIRNAME;
use moon_logger::{color, debug};
use moon_platform::{Runtime, Version};
use moon_utils::{fs, path};
use proto_core::Proto;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct Toolchain {
    pub config: ToolchainConfig,

    /// The directory where toolchain artifacts are stored.
    /// This is typically ~/.moon.
    pub dir: PathBuf,

    /// Tools:
    pub node: ToolManager<NodeTool>,
}

impl Toolchain {
    pub fn load(config: &ToolchainConfig) -> Result<Toolchain, ToolchainError> {
        Toolchain::load_from(
            path::get_home_dir().ok_or(ToolchainError::MissingHomeDir)?,
            config,
        )
    }

    pub fn load_from<P: AsRef<Path>>(
        base_dir: P,
        config: &ToolchainConfig,
    ) -> Result<Toolchain, ToolchainError> {
        let dir = base_dir.as_ref().join(CONFIG_DIRNAME);

        debug!(
            target: "moon:toolchain",
            "Creating toolchain at {}",
            color::path(&dir)
        );

        fs::create_dir_all(&dir)?;

        let mut toolchain = Toolchain {
            config: config.to_owned(),
            dir,
            // Tools
            node: ToolManager::new(Runtime::Node(Version::default())),
        };
        let proto = toolchain.get_paths();

        if let Some(node_config) = &config.node {
            toolchain
                .node
                .register(NodeTool::new(&proto, node_config)?, true);
        }

        Ok(toolchain)
    }

    pub fn get_paths(&self) -> Proto {
        Proto::new(&self.dir)
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
