use super::bin_config::BinEntry;
use crate::config_struct;
use schematic::Config;
use version_spec::UnresolvedVersionSpec;
use warpgate_api::PluginLocator;

config_struct!(
    /// Configures and enables the Deno platform.
    /// Docs: https://moonrepo.dev/docs/config/toolchain#deno
    #[derive(Config)]
    pub struct DenoConfig {
        /// List of binaries to install into the environment using `deno install`.
        #[setting(nested)]
        pub bins: Vec<BinEntry>,

        /// Relative path to a dependency management file. Used for content hashing.
        #[setting(default = "deps.ts")]
        pub deps_file: String,

        /// List of arguments to append to `deno install` commands.
        pub install_args: Vec<String>,

        /// Requires and forces the use of `deno.lock` files.
        pub lockfile: bool,

        /// Location of the WASM plugin to use for Deno support.
        pub plugin: Option<PluginLocator>,

        /// The version of Deno to download, install, and run `deno` tasks with.
        #[setting(env = "MOON_DENO_VERSION")]
        pub version: Option<UnresolvedVersionSpec>,
    }
);
