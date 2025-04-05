use crate::{config_struct, config_unit_enum};
use schematic::{Config, ConfigEnum};
use version_spec::UnresolvedVersionSpec;
use warpgate_api::PluginLocator;

#[cfg(feature = "proto")]
use crate::inherit_tool;

config_unit_enum!(
    /// The available package managers for Python.
    #[derive(ConfigEnum)]
    pub enum PythonPackageManager {
        #[default]
        Pip,
        Uv,
    }
);

config_struct!(
    #[derive(Config)]
    pub struct PipConfig {
        /// List of arguments to append to `pip install` commands.
        pub install_args: Vec<String>,
    }
);

config_struct!(
    #[derive(Config)]
    pub struct UvConfig {
        /// Location of the WASM plugin to use for uv support.
        pub plugin: Option<PluginLocator>,

        /// List of arguments to append to `uv sync` commands.
        pub sync_args: Vec<String>,

        /// The version of uv to download, install, and run `uv` tasks with.
        #[setting(env = "MOON_UV_VERSION")]
        pub version: Option<UnresolvedVersionSpec>,
    }
);

config_struct!(
    #[derive(Config)]
    pub struct PythonConfig {
        /// The package manager to use for installing dependencies and managing
        /// the virtual environment.
        pub package_manager: PythonPackageManager,

        /// Options for pip, when used as a package manager.
        #[setting(nested)]
        pub pip: PipConfig,

        /// Location of the WASM plugin to use for Python support.
        pub plugin: Option<PluginLocator>,

        /// Assumes a workspace root virtual environment is used for dependencies.
        /// Can be used to support the "one version policy" pattern.
        #[setting(alias = "rootRequirementsOnly")]
        pub root_venv_only: bool,

        /// Options for uv, when used as a package manager.
        #[setting(nested)]
        pub uv: Option<UvConfig>,

        /// Defines the virtual environment name, which will be created in the workspace root.
        /// Project dependencies will be installed into this.
        #[setting(default = ".venv")]
        pub venv_name: String,

        /// The version of Python to download, install, and run `python` tasks with.
        #[setting(env = "MOON_PYTHON_VERSION")]
        pub version: Option<UnresolvedVersionSpec>,
    }
);

#[cfg(feature = "proto")]
impl PythonConfig {
    inherit_tool!(UvConfig, uv, "uv", inherit_proto_uv);

    pub fn inherit_proto(&mut self, proto_config: &proto_core::ProtoConfig) -> miette::Result<()> {
        match &self.package_manager {
            PythonPackageManager::Pip => {
                // Built-in
            }
            PythonPackageManager::Uv => {
                if self.uv.is_none() {
                    self.uv = Some(UvConfig::default());
                }

                self.inherit_proto_uv(proto_config)?;
            }
        }

        Ok(())
    }
}
