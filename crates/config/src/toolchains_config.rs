use crate::patterns::{merge_iter, merge_plugin_partials};
use crate::toolchain::*;
use crate::{config_enum, config_struct};
use miette::IntoDiagnostic;
use moon_common::{Id, IdExt};
use rustc_hash::FxHashMap;
use schematic::{Config, Schematic, validate};
use serde_json::Value;
use std::collections::BTreeMap;
use std::env;
use version_spec::UnresolvedVersionSpec;
use warpgate_api::PluginLocator;

config_enum!(
    /// The strategy in which to inherit a version from `.prototools`.
    #[derive(Schematic)]
    #[serde(untagged)]
    pub enum ToolchainPluginVersionFrom {
        Enabled(bool),
        Id(String),
    }
);

impl Default for ToolchainPluginVersionFrom {
    fn default() -> Self {
        Self::Enabled(true)
    }
}

config_struct!(
    /// Configures an individual toolchain.
    #[derive(Config)]
    #[config(allow_unknown_fields)]
    pub struct ToolchainPluginConfig {
        /// Inherit aliases (name derived from a manifest) for all
        /// projects associated with this toolchain.
        /// @since 2.1.0
        #[setting(default = true)]
        pub inherit_aliases: bool,

        /// Run the `InstallDependencies` actions for each running task
        /// when changes to lockfiles and manifests are detected.
        /// @since 2.1.0
        #[setting(default = true)]
        pub install_dependencies: bool,

        /// Location of the WASM plugin to use.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub plugin: Option<PluginLocator>,

        /// The version of the toolchain to download and install.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub version: Option<UnresolvedVersionSpec>,

        /// Inherit the version from the root `.prototools`.
        /// When true, matches using the same identifier, otherwise a
        /// string can be provided for a custom identifier.
        pub version_from_prototools: ToolchainPluginVersionFrom,

        /// Arbitrary configuration that'll be passed to the WASM plugin.
        #[setting(flatten, merge = merge_iter)]
        pub config: BTreeMap<String, Value>,
    }
);

impl ToolchainPluginConfig {
    pub fn to_json(&self) -> Value {
        let mut data = Value::Object(self.config.clone().into_iter().collect());

        if let Some(version) = &self.version {
            data["version"] = Value::String(version.to_string());
        }

        data
    }
}

config_struct!(
    /// Configures all toolchains.
    /// Docs: https://moonrepo.dev/docs/config/toolchain
    #[derive(Config)]
    #[config(allow_unknown_fields)]
    pub struct ToolchainsConfig {
        #[setting(default = "./cache/schemas/toolchains.json", rename = "$schema")]
        pub schema: String,

        /// Extends one or many toolchain configuration files.
        /// Supports a relative file path or a secure URL.
        /// @since 1.12.0
        #[setting(extend, validate = validate::extends_from)]
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub extends: Option<schematic::ExtendsFrom>,

        /// Configures moon itself.
        #[setting(nested)]
        pub moon: MoonConfig,

        /// Configures how moon integrates with proto.
        #[setting(nested)]
        pub proto: ProtoConfig,

        /// Configures and integrates toolchains into the system using
        /// a unique identifier.
        #[setting(flatten, nested, merge = merge_plugin_partials)]
        pub plugins: FxHashMap<Id, ToolchainPluginConfig>,
    }
);

impl ToolchainsConfig {
    pub fn get_enabled(&self) -> Vec<Id> {
        let mut tools = self.plugins.keys().cloned().collect::<Vec<_>>();
        tools.push(Id::raw("system"));
        tools
    }

    pub fn get_plugin_config(&self, id: impl AsRef<str>) -> Option<&ToolchainPluginConfig> {
        let (stable_id, unstable_id) = Id::stable_and_unstable(id);

        self.plugins
            .get(&stable_id)
            .or_else(|| self.plugins.get(&unstable_id))
    }

    pub fn inherit_versions_from_env_vars(&mut self) -> miette::Result<()> {
        for (id, config) in &mut self.plugins {
            if let Ok(version) = env::var(format!("MOON_{}_VERSION", id.to_env_var())) {
                config.version = Some(UnresolvedVersionSpec::parse(version).into_diagnostic()?);
            }
        }

        Ok(())
    }

    pub fn requires_proto(&self) -> bool {
        for config in self.plugins.values() {
            if config.version.is_some() {
                return true;
            }
        }

        false
    }

    pub fn should_invalidate(&self, other: &Self) -> bool {
        self.plugins != other.plugins
    }
}
