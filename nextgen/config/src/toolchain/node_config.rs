use schematic::{derive_enum, Config, ConfigEnum};
use version_spec::UnresolvedVersionSpec;
use warpgate_api::PluginLocator;

#[cfg(feature = "proto")]
use crate::{inherit_tool, inherit_tool_required};

derive_enum!(
    #[derive(ConfigEnum, Copy, Default)]
    pub enum NodeVersionFormat {
        File,         // file:..
        Link,         // link:..
        Star,         // *
        Version,      // 0.0.0
        VersionCaret, // ^0.0.0
        VersionTilde, // ~0.0.0
        #[default]
        Workspace, // workspace:*
        WorkspaceCaret, // workspace:^
        WorkspaceTilde, // workspace:~
    }
);

impl NodeVersionFormat {
    pub fn get_default_for(&self, pm: &NodePackageManager) -> Self {
        match pm {
            NodePackageManager::Npm => Self::File,
            _ => Self::Workspace,
        }
    }

    pub fn get_prefix(&self) -> String {
        match self {
            NodeVersionFormat::File => "file:".into(),
            NodeVersionFormat::Link => "link:".into(),
            NodeVersionFormat::Star => "*".into(),
            NodeVersionFormat::Version => "".into(),
            NodeVersionFormat::VersionCaret => "^".into(),
            NodeVersionFormat::VersionTilde => "~".into(),
            NodeVersionFormat::Workspace => "workspace:*".into(),
            NodeVersionFormat::WorkspaceCaret => "workspace:^".into(),
            NodeVersionFormat::WorkspaceTilde => "workspace:~".into(),
        }
    }

    pub fn is_supported_by(&self, pm: &NodePackageManager) -> bool {
        match pm {
            NodePackageManager::Bun => !matches!(self, Self::WorkspaceCaret | Self::WorkspaceTilde),
            NodePackageManager::Npm => !matches!(
                self,
                Self::Link | Self::Workspace | Self::WorkspaceCaret | Self::WorkspaceTilde
            ),
            NodePackageManager::Pnpm => true,
            NodePackageManager::Yarn => true,
        }
    }
}

derive_enum!(
    #[derive(ConfigEnum, Copy, Default)]
    pub enum NodePackageManager {
        Bun,
        #[default]
        Npm,
        Pnpm,
        Yarn,
    }
);

derive_enum!(
    #[derive(ConfigEnum, Copy, Default)]
    pub enum NodeVersionManager {
        Nodenv,
        #[default]
        Nvm,
    }
);

#[derive(Clone, Config, Debug)]
pub struct BunpmConfig {
    pub plugin: Option<PluginLocator>,

    #[setting(env = "MOON_BUN_VERSION")]
    pub version: Option<UnresolvedVersionSpec>,
}

#[derive(Clone, Config, Debug)]
pub struct NpmConfig {
    pub plugin: Option<PluginLocator>,

    #[setting(env = "MOON_NPM_VERSION")]
    pub version: Option<UnresolvedVersionSpec>,
}

#[derive(Clone, Config, Debug)]
pub struct PnpmConfig {
    pub plugin: Option<PluginLocator>,

    #[setting(env = "MOON_PNPM_VERSION")]
    pub version: Option<UnresolvedVersionSpec>,
}

#[derive(Clone, Config, Debug)]
pub struct YarnConfig {
    pub plugin: Option<PluginLocator>,

    pub plugins: Vec<String>,

    #[setting(env = "MOON_YARN_VERSION")]
    pub version: Option<UnresolvedVersionSpec>,
}

/// Docs: https://moonrepo.dev/docs/config/toolchain#node
#[derive(Clone, Config, Debug)]
pub struct NodeConfig {
    #[setting(default = true)]
    pub add_engines_constraint: bool,

    pub bin_exec_args: Vec<String>,

    #[setting(nested)]
    pub bun: Option<BunpmConfig>,

    #[setting(default = true)]
    pub dedupe_on_lockfile_change: bool,

    pub dependency_version_format: NodeVersionFormat,

    pub infer_tasks_from_scripts: bool,

    #[setting(nested)]
    pub npm: NpmConfig,

    pub package_manager: NodePackageManager,

    #[setting(default = ".", skip)]
    pub packages_root: String,

    pub plugin: Option<PluginLocator>,

    #[setting(nested)]
    pub pnpm: Option<PnpmConfig>,

    pub root_package_only: bool,

    #[setting(default = true)]
    pub sync_project_workspace_dependencies: bool,

    pub sync_version_manager_config: Option<NodeVersionManager>,

    #[setting(env = "MOON_NODE_VERSION")]
    pub version: Option<UnresolvedVersionSpec>,

    #[setting(nested)]
    pub yarn: Option<YarnConfig>,
}

#[cfg(feature = "proto")]
impl NodeConfig {
    inherit_tool_required!(NpmConfig, npm, "npm", inherit_proto_npm);

    inherit_tool!(BunpmConfig, bun, "bun", inherit_proto_bun);

    inherit_tool!(PnpmConfig, pnpm, "pnpm", inherit_proto_pnpm);

    inherit_tool!(YarnConfig, yarn, "yarn", inherit_proto_yarn);

    pub fn inherit_proto(&mut self, proto_config: &ProtoConfig) -> miette::Result<()> {
        use moon_common::color;
        use proto_core::ProtoConfig;
        use tracing::debug;

        match &self.package_manager {
            NodePackageManager::Bun => {
                if self.bun.is_none() {
                    self.bun = Some(BunpmConfig::default());
                }

                self.inherit_proto_bun(proto_config)?;
            }
            NodePackageManager::Npm => {
                self.inherit_proto_npm(proto_config)?;
            }
            NodePackageManager::Pnpm => {
                if self.pnpm.is_none() {
                    self.pnpm = Some(PnpmConfig::default());
                }

                self.inherit_proto_pnpm(proto_config)?;
            }
            NodePackageManager::Yarn => {
                if self.yarn.is_none() {
                    self.yarn = Some(YarnConfig::default());
                }

                self.inherit_proto_yarn(proto_config)?;
            }
        };

        if !self
            .dependency_version_format
            .is_supported_by(&self.package_manager)
        {
            let new_format = self
                .dependency_version_format
                .get_default_for(&self.package_manager);

            debug!(
                "{} for {} is not supported by {}, changing to {}",
                color::symbol(self.dependency_version_format.to_string()),
                color::property("node.dependencyVersionFormat"),
                self.package_manager.to_string(),
                color::symbol(new_format.to_string()),
            );

            self.dependency_version_format = new_format;
        }

        Ok(())
    }
}
