// use super::bin_config::BinEntry;
use schematic::Config;
use serde::Serialize;
use version_spec::UnresolvedVersionSpec;
use warpgate_api::PluginLocator;

#[derive(Clone, Config, Debug, PartialEq, Serialize)]
pub struct PipConfig {
    /// List of arguments to append to `pip install` commands.
    pub install_args: Option<Vec<String>>,

    /// The version of pip to download, install, and run `pip` tasks with.
    pub version: Option<UnresolvedVersionSpec>,
}

#[derive(Clone, Config, Debug, PartialEq)]
pub struct PythonConfig {
    /// Location of the WASM plugin to use for Python support.
    pub plugin: Option<PluginLocator>,

    /// Options for pip, when used as a package manager.
    #[setting(nested)]
    pub pip: Option<PipConfig>,

    #[setting(default = ".venv", skip)]
    pub venv_name: String,

    /// The version of Python to download, install, and run `python` tasks with.
    #[setting(env = "MOON_PYTHON_VERSION")]
    pub version: Option<UnresolvedVersionSpec>,
}
