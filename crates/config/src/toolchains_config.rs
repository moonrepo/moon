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
}

#[cfg(feature = "proto")]
impl ToolchainsConfig {
    pub fn requires_proto(&self) -> bool {
        for config in self.plugins.values() {
            if config.version.is_some() {
                return true;
            }
        }

        false
    }

    pub fn get_plugin_locator(id: &Id) -> Option<proto_core::PluginLocator> {
        use proto_core::warpgate::find_debug_locator_with_url_fallback as locate;

        match id.as_str() {
            "bun" => Some(locate("bun_toolchain", "1.0.2")),
            "deno" => Some(locate("deno_toolchain", "1.0.3")),
            "go" => Some(locate("go_toolchain", "1.0.3")),
            "javascript" => Some(locate("javascript_toolchain", "1.0.4")),
            "node" => Some(locate("node_toolchain", "1.0.2")),
            "npm" => Some(locate("node_depman_toolchain", "1.0.2")),
            "pnpm" => Some(locate("node_depman_toolchain", "1.0.2")),
            "rust" => Some(locate("rust_toolchain", "1.0.4")),
            "system" => Some(locate("system_toolchain", "1.0.2")),
            "typescript" => Some(locate("typescript_toolchain", "1.0.3")),
            "unstable_python" => Some(locate("python_toolchain", "0.1.2")),
            "unstable_pip" => Some(locate("python_pip_toolchain", "0.1.2")),
            "unstable_uv" => Some(locate("python_uv_toolchain", "0.1.2")),
            "yarn" => Some(locate("node_depman_toolchain", "1.0.2")),
            _ => None,
        }
    }

    pub fn inherit_defaults(
        &mut self,
        proto_config: &proto_core::ProtoConfig,
    ) -> miette::Result<()> {
        self.inherit_proto_versions_for_plugins(proto_config)?;
        self.inherit_default_plugins();
        self.inherit_plugin_locators()?;

        Ok(())
    }

    pub fn inherit_proto_versions_for_plugins(
        &mut self,
        proto_config: &proto_core::ProtoConfig,
    ) -> miette::Result<()> {
        use moon_common::color;
        use proto_core::ToolContext;
        use tracing::trace;

        for (id, config) in &mut self.plugins {
            if config.version.is_some() {
                continue;
            }

            let proto_id = match &config.version_from_prototools {
                ToolchainPluginVersionFrom::Enabled(enabled) => {
                    if *enabled {
                        id.as_str().strip_prefix("unstable_").unwrap_or(id.as_str())
                    } else {
                        continue;
                    }
                }
                ToolchainPluginVersionFrom::Id(custom_id) => custom_id,
            };
            let proto_context = ToolContext::parse(proto_id).unwrap();

            if let Some(version) = proto_config.versions.get(&proto_context) {
                trace!(
                    "Inheriting {} version {} from .prototools",
                    color::id(id),
                    version
                );

                config.version = Some(version.req.to_owned());
            }
        }

        Ok(())
    }

    pub fn inherit_default_plugins(&mut self) {
        self.plugins.entry(Id::raw("system")).or_default();
    }

    pub fn inherit_test_plugins(&mut self) -> miette::Result<()> {
        for id in [
            "tc-tier1",
            "tc-tier2",
            "tc-tier2-reqs",
            "tc-tier2-setup-env",
            "tc-tier3",
            "tc-tier3-reqs",
        ] {
            self.plugins.entry(Id::raw(id)).or_default();
        }

        Ok(())
    }

    pub fn inherit_test_builtin_plugins(&mut self) -> miette::Result<()> {
        // We don't need all package managers
        for id in [
            "bun",
            "deno",
            "go",
            "javascript",
            "node",
            "npm",
            "rust",
            "system",
            "typescript",
            "unstable_python",
            "unstable_pip",
        ] {
            self.plugins.entry(Id::raw(id)).or_default();
        }

        Ok(())
    }

    pub fn inherit_plugin_locators(&mut self) -> miette::Result<()> {
        use schematic::{ConfigError, Path, PathSegment, ValidateError, ValidatorError};

        for (id, config) in self.plugins.iter_mut() {
            if config.plugin.is_some() {
                continue;
            }

            match id.as_str() {
                "bun" | "deno" | "go" | "javascript" | "node" | "npm" | "pnpm" | "rust"
                | "system" | "typescript" | "unstable_python" | "unstable_pip" | "unstable_uv"
                | "yarn" => {
                    config.plugin = Self::get_plugin_locator(id);
                }
                #[cfg(debug_assertions)]
                "tc-tier1" | "tc-tier2" | "tc-tier2-reqs" | "tc-tier2-setup-env" | "tc-tier3"
                | "tc-tier3-reqs" => {
                    use proto_core::warpgate::find_debug_locator;

                    config.plugin = Some(
                        find_debug_locator(&id.replace("-", "_"))
                            .expect("Development plugins missing, build with `just build-wasm`!"),
                    );

                    if id.contains("tc-tier3") {
                        config.version = UnresolvedVersionSpec::parse("1.2.3").ok();
                    }
                }
                other => {
                    return Err(ConfigError::Validator {
                        location: ".moon/toolchains.*".into(),
                        error: Box::new(ValidatorError {
                            errors: vec![ValidateError {
                                message:
                                    "a locator is required for plugins; accepts file paths and URLs"
                                        .into(),
                                path: Path::new(vec![
                                    PathSegment::Key(other.to_string()),
                                    PathSegment::Key("plugin".into()),
                                ]),
                            }],
                        }),
                        help: None,
                    }
                    .into());
                }
            };
        }

        Ok(())
    }
}
