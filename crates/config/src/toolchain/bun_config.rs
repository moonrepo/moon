use super::node_config::NodeVersionFormat;
use crate::config_struct;
use schematic::Config;
use version_spec::UnresolvedVersionSpec;
use warpgate_api::PluginLocator;

config_struct!(
    /// Configures and enables the Bun platform.
    /// Docs: https://moonrepo.dev/docs/config/toolchain#bun
    #[derive(Config)]
    pub struct BunConfig {
        /// The dependency version format to use when syncing projects
        /// as dependencies.
        pub dependency_version_format: NodeVersionFormat,

        /// Automatically infer moon tasks from `package.json` scripts.
        pub infer_tasks_from_scripts: bool,

        /// List of arguments to append to `bun install` commands.
        pub install_args: Vec<String>,

        /// The relative root of the packages workspace. Defaults to moon's
        /// workspace root, but should be defined when nested.
        #[setting(default = ".", skip)]
        pub packages_root: String,

        /// Location of the WASM plugin to use for Bun support.
        pub plugin: Option<PluginLocator>,

        /// Assumes only the root `package.json` is used for dependencies.
        /// Can be used to support the "one version policy" pattern.
        pub root_package_only: bool,

        /// Automatically syncs moon project-to-project relationships as
        /// dependencies for each `package.json` in the workspace.
        #[setting(default = true)]
        pub sync_project_workspace_dependencies: bool,

        /// The version of Bun to download, install, and run `bun` tasks with.
        #[setting(env = "MOON_BUN_VERSION")]
        pub version: Option<UnresolvedVersionSpec>,
    }
);
