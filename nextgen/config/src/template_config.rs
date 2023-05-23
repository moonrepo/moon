// template.yml

use moon_common::consts;
use rustc_hash::FxHashMap;
use schematic::{color, config_enum, validate, Config, ConfigError, ConfigLoader};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TemplateVariableSetting<T> {
    pub default: T,
    pub prompt: Option<String>,
    pub required: Option<bool>,
}

config_enum!(
    #[serde(
        untagged,
        expecting = "expected a value string or value object with label"
    )]
    pub enum TemplateVariableEnumValue {
        String(String),
        Object { label: String, value: String },
    }
);

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TemplateVariableEnumSetting {
    pub default: String,
    pub multiple: Option<bool>,
    pub prompt: String,
    pub values: Vec<TemplateVariableEnumValue>,
}

config_enum!(
    #[serde(tag = "type")]
    pub enum TemplateVariable {
        Boolean(TemplateVariableSetting<bool>),
        Enum(TemplateVariableEnumSetting),
        Number(TemplateVariableSetting<usize>),
        // NumberList(TemplateVariableConfig<Vec<usize>>),
        String(TemplateVariableSetting<String>),
        // StringList(TemplateVariableConfig<Vec<String>>),
    }
);

/// Docs: https://moonrepo.dev/docs/config/template
#[derive(Config)]
pub struct TemplateConfig {
    #[setting(
        default = "https://moonrepo.dev/schemas/template.json",
        rename = "$schema"
    )]
    pub schema: String,

    #[setting(validate = validate::not_empty)]
    pub description: String,

    #[setting(validate = validate::not_empty)]
    pub title: String,

    pub variables: FxHashMap<String, TemplateVariable>,
}

impl TemplateConfig {
    pub fn load<R: AsRef<Path>, P: AsRef<Path>>(
        workspace_root: R,
        path: P,
    ) -> Result<TemplateConfig, ConfigError> {
        let workspace_root = workspace_root.as_ref();
        let path = path.as_ref();

        let result = ConfigLoader::<TemplateConfig>::yaml()
            .label(color::path(
                if let Ok(relative_path) = path.strip_prefix(workspace_root) {
                    relative_path
                } else {
                    path
                },
            ))
            .file(path)?
            .load()?;

        Ok(result.config)
    }

    pub fn load_from<R: AsRef<Path>, P: AsRef<Path>>(
        workspace_root: R,
        template_root: P,
    ) -> Result<TemplateConfig, ConfigError> {
        Self::load(
            workspace_root,
            template_root
                .as_ref()
                .join(consts::CONFIG_TEMPLATE_FILENAME),
        )
    }
}
