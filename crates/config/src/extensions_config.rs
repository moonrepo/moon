use crate::config_struct;
use crate::patterns::{merge_iter, merge_plugin_partials};
use moon_common::Id;
use rustc_hash::FxHashMap;
use schematic::{Config, validate};
use serde_json::Value;
use warpgate_api::PluginLocator;

config_struct!(
    /// Configures an individual extension.
    #[derive(Config)]
    #[config(allow_unknown_fields)]
    pub struct ExtensionPluginConfig {
        /// Location of the WASM plugin to use.
        pub plugin: Option<PluginLocator>,

        /// Arbitrary configuration that'll be passed to the WASM plugin.
        #[setting(flatten, merge = merge_iter)]
        pub config: FxHashMap<String, serde_json::Value>,
    }
);

impl ExtensionPluginConfig {
    pub fn get_plugin_locator(&self) -> &PluginLocator {
        self.plugin.as_ref().unwrap()
    }

    pub fn to_json(&self) -> Value {
        Value::Object(self.config.clone().into_iter().collect())
    }
}

config_struct!(
    /// Configures all extensions.
    #[derive(Config)]
    #[config(allow_unknown_fields)]
    pub struct ExtensionsConfig {
        #[setting(default = "./cache/schemas/extensions.json", rename = "$schema")]
        pub schema: String,

        /// Extends one or many extensions configuration files.
        /// Supports a relative file path or a secure URL.
        /// @since 2.0.0
        #[setting(extend, validate = validate::extends_from)]
        pub extends: Option<schematic::ExtendsFrom>,

        /// Configures and integrates extensions into the system using
        /// a unique identifier.
        #[setting(flatten, nested, merge = merge_plugin_partials)]
        pub plugins: FxHashMap<Id, ExtensionPluginConfig>,
    }
);

impl ExtensionsConfig {
    pub fn get_plugin_config(&self, id: impl AsRef<str>) -> Option<&ExtensionPluginConfig> {
        self.plugins.get(id.as_ref())
    }
}

#[cfg(feature = "proto")]
impl ExtensionsConfig {
    pub fn get_plugin_locator(id: &Id) -> Option<proto_core::PluginLocator> {
        use proto_core::warpgate::find_debug_locator_with_url_fallback as locate;

        match id.as_str() {
            "download" => Some(locate("download_extension", "1.0.0")),
            "migrate-nx" => Some(locate("migrate_nx_extension", "1.0.0")),
            "migrate-turborepo" => Some(locate("migrate_turborepo_extension", "1.0.0")),
            "unpack" => Some(locate("unpack_extension", "1.0.0")),
            _ => None,
        }
    }

    pub fn inherit_defaults(&mut self) -> miette::Result<()> {
        self.inherit_default_plugins();
        self.inherit_plugin_locators()?;

        Ok(())
    }

    pub fn inherit_default_plugins(&mut self) {
        // N/A
    }

    pub fn inherit_test_plugins(&mut self) -> miette::Result<()> {
        for id in ["ext-sync", "ext-task"] {
            self.plugins.entry(Id::raw(id)).or_default();
        }

        Ok(())
    }

    pub fn inherit_test_builtin_plugins(&mut self) {
        for id in ["download", "migrate-nx", "migrate-turborepo"] {
            self.plugins.entry(Id::raw(id)).or_default();
        }
    }

    pub fn inherit_plugin_locators(&mut self) -> miette::Result<()> {
        use schematic::{ConfigError, Path, PathSegment, ValidateError, ValidatorError};

        for (id, config) in self.plugins.iter_mut() {
            if config.plugin.is_some() {
                continue;
            }

            match id.as_str() {
                "download" | "migrate-nx" | "migrate-turborepo" => {
                    config.plugin = Self::get_plugin_locator(id);
                }
                #[cfg(debug_assertions)]
                "ext-sync" | "ext-task" => {
                    use proto_core::warpgate::find_debug_locator;

                    config.plugin = Some(
                        find_debug_locator(&id.replace("-", "_"))
                            .expect("Development plugins missing, build with `just build-wasm`!"),
                    );
                }
                other => {
                    return Err(ConfigError::Validator {
                        location: ".moon/extensions.*".into(),
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
