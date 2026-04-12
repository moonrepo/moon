use moon_common::Id;
use moon_config::ExtensionsConfig;
use proto_core::{
    PluginLocator,
    warpgate::{find_debug_locator, find_debug_locator_with_url_fallback as locate},
};
use schematic::{ConfigError, Path, PathSegment, ValidateError, ValidatorError};

pub struct ExtensionsConfigExt;

impl ExtensionsConfigExt {
    pub fn get_plugin_locator(id: &Id) -> Option<PluginLocator> {
        match id.as_str() {
            "download" => Some(locate("download_extension", "1.0.2")),
            "migrate-nx" => Some(locate("migrate_nx_extension", "1.0.3")),
            "migrate-turborepo" => Some(locate("migrate_turborepo_extension", "1.0.3")),
            "unpack" => Some(locate("unpack_extension", "1.0.2")),
            _ => None,
        }
    }

    pub fn inherit_defaults(config: &mut ExtensionsConfig) -> miette::Result<()> {
        Self::inherit_default_plugins(config);
        Self::inherit_plugin_locators(config)?;

        Ok(())
    }

    pub fn inherit_default_plugins(_config: &mut ExtensionsConfig) {
        // N/A
    }

    pub fn inherit_test_plugins(config: &mut ExtensionsConfig) -> miette::Result<()> {
        for id in ["ext-sync", "ext-task"] {
            config.plugins.entry(Id::raw(id)).or_default();
        }

        Ok(())
    }

    pub fn inherit_test_builtin_plugins(config: &mut ExtensionsConfig) {
        for id in ["download", "migrate-nx", "migrate-turborepo"] {
            config.plugins.entry(Id::raw(id)).or_default();
        }
    }

    pub fn inherit_plugin_locators(config: &mut ExtensionsConfig) -> miette::Result<()> {
        for (id, config) in config.plugins.iter_mut() {
            if config.plugin.is_some() {
                continue;
            }

            match id.as_str() {
                "download" | "migrate-nx" | "migrate-turborepo" => {
                    config.plugin = Self::get_plugin_locator(id);
                }
                #[cfg(debug_assertions)]
                "ext-sync" | "ext-task" => {
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
