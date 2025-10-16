use crate::patterns::{merge_iter, merge_plugin_partials};
use crate::toolchain::*;
use crate::{config_enum, config_struct};
use moon_common::{Id, IdExt};
use rustc_hash::FxHashMap;
use schematic::{Config, Schematic, validate};
use serde_json::Value;
use std::collections::BTreeMap;
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
        pub plugin: Option<PluginLocator>,

        /// The version of the toolchain to download and install.
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
    pub struct ToolchainConfig {
        #[setting(
            default = "https://moonrepo.dev/schemas/toolchain.json",
            rename = "$schema"
        )]
        pub schema: String,

        /// Extends one or many toolchain configuration files.
        /// Supports a relative file path or a secure URL.
        /// @since 1.12.0
        #[setting(extend, validate = validate::extends_from)]
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

impl ToolchainConfig {
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

    pub fn is_plugin(&self, id: &str) -> bool {
        self.plugins.contains_key(id)
    }
}

#[cfg(feature = "proto")]
impl ToolchainConfig {
    pub fn requires_proto(&self) -> bool {
        for config in self.plugins.values() {
            if config.version.is_some() {
                return true;
            }
        }

        false
    }

    pub fn get_plugin_locator(id: &Id) -> Option<proto_core::PluginLocator> {
        use proto_core::warpgate::find_debug_locator_with_url_fallback;

        // TODO remove once v2 plugins are published
        let locate = |name: &str, version: &str| {
            #[cfg(debug_assertions)]
            {
                use std::env;
                use std::path::PathBuf;

                let prebuilts_dir = env::var("WASM_PREBUILTS_DIR")
                    .map(PathBuf::from)
                    .unwrap_or_else(|_| {
                        let root = env::current_dir().unwrap();

                        // repo root
                        if root.join("wasm/prebuilts").exists() {
                            root.join("wasm/prebuilts")
                        }
                        // within a crate
                        else {
                            root.join("../../wasm/prebuilts")
                        }
                    });
                let wasm_path = prebuilts_dir.join(format!("{name}.wasm"));

                if wasm_path.exists() {
                    return PluginLocator::File(Box::new(proto_core::FileLocator {
                        file: format!("file://{}", wasm_path.display()),
                        path: Some(wasm_path),
                    }));
                }
            }

            find_debug_locator_with_url_fallback(name, version)
        };

        match id.as_str() {
            "bun" => Some(locate("bun_toolchain", "0.2.0")),
            "deno" => Some(locate("deno_toolchain", "0.1.0")),
            "go" => Some(locate("go_toolchain", "0.2.0")),
            "javascript" => Some(locate("javascript_toolchain", "0.2.2")),
            "node" => Some(locate("node_toolchain", "0.2.0")),
            "npm" => Some(locate("node_depman_toolchain", "0.2.0")),
            "pnpm" => Some(locate("node_depman_toolchain", "0.2.0")),
            "rust" => Some(locate("rust_toolchain", "0.3.0")),
            "system" => Some(locate("system_toolchain", "0.0.1")),
            "typescript" => Some(locate("typescript_toolchain", "0.3.0")),
            "yarn" => Some(locate("node_depman_toolchain", "0.2.0")),
            _ => None,
        }
    }

    pub fn inherit_proto(&mut self, proto_config: &proto_core::ProtoConfig) -> miette::Result<()> {
        self.inherit_proto_for_plugins(proto_config)?;
        self.inherit_system_plugin();
        self.inherit_plugin_locators()?;

        Ok(())
    }

    pub fn inherit_proto_for_plugins(
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

    pub fn inherit_system_plugin(&mut self) {
        self.plugins.entry(Id::raw("system")).or_default();
    }

    pub fn inherit_default_plugins(&mut self) -> miette::Result<()> {
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
            // We only need 1 package manager while testing!
            // "pnpm",
            // "yarn",
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
                | "system" | "typescript" | "yarn" => {
                    config.plugin = Self::get_plugin_locator(id);
                }
                other => {
                    return Err(ConfigError::Validator {
                        location: ".moon/toolchain.yml".into(),
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
