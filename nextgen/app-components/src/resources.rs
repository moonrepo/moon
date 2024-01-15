use moon_config::{InheritedTasksManager, ToolchainConfig, WorkspaceConfig};
use moon_extension_plugin::ExtensionPlugin;
use moon_plugin::PluginRegistry;
use proto_core::ProtoConfig;
use semver::Version;
use starbase::Resource;
use std::path::{Path, PathBuf};

#[derive(Debug, Resource)]
pub struct AppInfo {
    /// The executed moon binary that kicked off the app process.
    pub current_exe: Option<PathBuf>,

    /// Is running with a global moon binary.
    pub global: bool,

    /// The moon binary that is currently running. This may be different
    /// than `current_exe` if we detect a local binary to use instead of
    /// the running global.
    pub running_exe: Option<PathBuf>,

    /// Current versio of moon.
    pub version: Version,
}

#[derive(Resource)]
pub struct ExtensionRegistry(pub PluginRegistry<ExtensionPlugin>);

impl ExtensionRegistry {
    pub fn new(plugins_dir: &Path, temp_dir: &Path) -> Self {
        Self(PluginRegistry::new(plugins_dir, temp_dir))
    }
}

#[derive(Debug, Resource)]
pub struct Tasks {
    pub manager: InheritedTasksManager,
}

#[derive(Debug, Resource)]
pub struct Toolchain {
    pub config: ToolchainConfig,
    pub proto_config: ProtoConfig,
    pub proto_home: PathBuf,
}

#[derive(Debug, Resource)]
pub struct Workspace {
    pub config: WorkspaceConfig,
    pub telemetry: bool,
}
