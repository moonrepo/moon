use crate::{config_struct, config_unit_enum};
use schematic::{Config, ConfigEnum};
use version_spec::UnresolvedVersionSpec;
use warpgate_api::PluginLocator;

#[cfg(feature = "proto")]
use crate::{inherit_tool, inherit_tool_required};

config_unit_enum!(
    /// Formats that a `package.json` version dependency can be.
    #[derive(ConfigEnum)]
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

config_unit_enum!(
    /// The available package managers for Node.js.
    #[derive(ConfigEnum)]
    pub enum NodePackageManager {
        Bun,
        #[default]
        Npm,
        Pnpm,
        Yarn,
    }
);

config_unit_enum!(
    /// The available version managers for Node.js.
    #[derive(ConfigEnum)]
    pub enum NodeVersionManager {
        Nodenv,
        #[default]
        Nvm,
    }
);

config_struct!(
    /// Options for Bun, when used as a package manager.
    #[derive(Config)]
    pub struct BunpmConfig {
        /// List of arguments to append to `bun install` commands.
        pub install_args: Vec<String>,

        /// Location of the WASM plugin to use for Bun support.
        pub plugin: Option<PluginLocator>,

        /// The version of Bun to download, install, and run `bun` tasks with.
        #[setting(env = "MOON_BUN_VERSION")]
        pub version: Option<UnresolvedVersionSpec>,
    }
);

config_struct!(
    /// Options for npm, when used as a package manager.
    #[derive(Config)]
    pub struct NpmConfig {
        /// List of arguments to append to `npm install` commands.
        #[setting(default = vec!["--no-audit".into(), "--no-fund".into()])]
        pub install_args: Vec<String>,

        /// Location of the WASM plugin to use for npm support.
        pub plugin: Option<PluginLocator>,

        /// The version of npm to download, install, and run `npm` tasks with.
        #[setting(env = "MOON_NPM_VERSION")]
        pub version: Option<UnresolvedVersionSpec>,
    }
);

config_struct!(
    /// Options for pnpm, when used as a package manager.
    #[derive(Config)]
    pub struct PnpmConfig {
        /// List of arguments to append to `pnpm install` commands.
        pub install_args: Vec<String>,

        /// Location of the WASM plugin to use for pnpm support.
        pub plugin: Option<PluginLocator>,

        /// The version of pnpm to download, install, and run `pnpm` tasks with.
        #[setting(env = "MOON_PNPM_VERSION")]
        pub version: Option<UnresolvedVersionSpec>,
    }
);

config_struct!(
    /// Options for Yarn, when used as a package manager.
    #[derive(Config)]
    pub struct YarnConfig {
        /// List of arguments to append to `yarn install` commands.
        pub install_args: Vec<String>,

        /// Location of the WASM plugin to use for Yarn support.
        pub plugin: Option<PluginLocator>,

        /// Plugins to automatically install for Yarn v2 and above.
        pub plugins: Vec<String>,

        /// The version of Yarn to download, install, and run `yarn` tasks with.
        #[setting(env = "MOON_YARN_VERSION")]
        pub version: Option<UnresolvedVersionSpec>,
    }
);

config_struct!(
    /// Configures and enables the Node.js platform.
    /// Docs: https://moonrepo.dev/docs/config/toolchain#node
    #[derive(Config)]
    pub struct NodeConfig {
        /// When `version` is defined, syncs the version as a constraint to
        /// `package.json` engines.
        #[setting(default = true)]
        pub add_engines_constraint: bool,

        /// Arguments to automatically pass to all tasks that execute the
        /// `node` binary.
        pub bin_exec_args: Vec<String>,

        /// Options for Bun, when used as a package manager.
        #[setting(nested)]
        pub bun: Option<BunpmConfig>,

        /// Automatically dedupes the lockfile when dependencies have changed.
        #[setting(default = true)]
        pub dedupe_on_lockfile_change: bool,

        /// The dependency version format to use when syncing projects
        /// as dependencies.
        pub dependency_version_format: NodeVersionFormat,

        /// Automatically infer moon tasks from `package.json` scripts.
        pub infer_tasks_from_scripts: bool,

        /// Options for npm, when used as a package manager.
        #[setting(nested)]
        pub npm: NpmConfig,

        /// The package manager to use for installing dependencies.
        pub package_manager: NodePackageManager,

        /// The relative root of the packages workspace. Defaults to moon's
        /// workspace root, but should be defined when nested.
        #[setting(default = ".", skip)]
        pub packages_root: String,

        /// Location of the WASM plugin to use for Node.js support.
        pub plugin: Option<PluginLocator>,

        /// Options for pnpm, when used as a package manager.
        #[setting(nested)]
        pub pnpm: Option<PnpmConfig>,

        /// Assumes only the root `package.json` is used for dependencies.
        /// Can be used to support the "one version policy" pattern.
        pub root_package_only: bool,

        /// Automatically syncs the configured package manager version
        /// to the root `packageManager` field in `package.json`.
        #[setting(default = true)]
        pub sync_package_manager_field: bool,

        /// Automatically syncs moon project-to-project relationships as
        /// dependencies for each `package.json` in the workspace.
        #[setting(default = true)]
        pub sync_project_workspace_dependencies: bool,

        /// When `version` is defined, syncs the version to the chosen config.
        pub sync_version_manager_config: Option<NodeVersionManager>,

        /// The version of Node.js to download, install, and run `node` tasks with.
        #[setting(env = "MOON_NODE_VERSION")]
        pub version: Option<UnresolvedVersionSpec>,

        /// Options for Yarn, when used as a package manager.
        #[setting(nested)]
        pub yarn: Option<YarnConfig>,
    }
);

#[cfg(feature = "proto")]
impl NodeConfig {
    inherit_tool_required!(NpmConfig, npm, "npm", inherit_proto_npm);

    inherit_tool!(BunpmConfig, bun, "bun", inherit_proto_bun);

    inherit_tool!(PnpmConfig, pnpm, "pnpm", inherit_proto_pnpm);

    inherit_tool!(YarnConfig, yarn, "yarn", inherit_proto_yarn);

    pub fn inherit_proto(&mut self, proto_config: &proto_core::ProtoConfig) -> miette::Result<()> {
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

            #[cfg(feature = "tracing")]
            {
                use moon_common::color;

                tracing::debug!(
                    "{} for {} is not supported by {}, changing to {}",
                    color::symbol(self.dependency_version_format.to_string()),
                    color::property("node.dependencyVersionFormat"),
                    self.package_manager.to_string(),
                    color::symbol(new_format.to_string()),
                );
            }

            self.dependency_version_format = new_format;
        }

        Ok(())
    }
}
