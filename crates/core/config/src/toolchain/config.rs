// .moon/toolchain.yml

use crate::errors::map_validation_errors_to_figment_errors;
use crate::helpers::gather_extended_sources;
use crate::toolchain::node::NodeConfig;
use crate::toolchain::typescript::TypeScriptConfig;
use crate::validators::validate_extends;
use crate::ConfigError;
use figment::{
    providers::{Format, Serialized, YamlExtended},
    Figment,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::env;
use std::path::PathBuf;
use validator::Validate;

/// Docs: https://moonrepo.dev/docs/config/toolchain
#[derive(Clone, Debug, Default, Deserialize, Eq, JsonSchema, PartialEq, Serialize, Validate)]
#[schemars(default)]
#[serde(default, rename_all = "camelCase")]
pub struct ToolchainConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(custom = "validate_extends")]
    pub extends: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate]
    pub node: Option<NodeConfig>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate]
    pub typescript: Option<TypeScriptConfig>,

    /// JSON schema URI.
    #[serde(skip, rename = "$schema")]
    pub schema: String,
}

impl ToolchainConfig {
    pub fn load(path: PathBuf) -> Result<ToolchainConfig, ConfigError> {
        let profile_name = "toolchain";
        let mut figment =
            Figment::from(Serialized::defaults(ToolchainConfig::default()).profile(profile_name));

        for source in gather_extended_sources(path)? {
            figment = figment.merge(YamlExtended::file(source).profile(profile_name));
        }

        let mut config = ToolchainConfig::load_config(figment.select(profile_name))?;
        config.extends = None;

        if let Some(node_config) = &mut config.node {
            // Versions from env vars should take precedence
            if let Ok(node_version) = env::var("MOON_NODE_VERSION") {
                node_config.version = Some(node_version);
            }

            if let Ok(npm_version) = env::var("MOON_NPM_VERSION") {
                node_config.npm.version = Some(npm_version);
            }

            if let Ok(pnpm_version) = env::var("MOON_PNPM_VERSION") {
                if let Some(pnpm_config) = &mut node_config.pnpm {
                    pnpm_config.version = Some(pnpm_version);
                }
            }

            if let Ok(yarn_version) = env::var("MOON_YARN_VERSION") {
                if let Some(yarn_config) = &mut node_config.yarn {
                    yarn_config.version = Some(yarn_version);
                }
            }
        }

        Ok(config)
    }

    fn load_config(figment: Figment) -> Result<ToolchainConfig, ConfigError> {
        let config: ToolchainConfig = figment.extract()?;

        if let Err(errors) = config.validate() {
            return Err(ConfigError::FailedValidation(
                map_validation_errors_to_figment_errors(&figment, &errors),
            ));
        }

        Ok(config)
    }
}
