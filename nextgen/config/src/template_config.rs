// template.yml

use moon_common::consts;
use rustc_hash::FxHashMap;
use schematic::{validate, Config, ConfigLoader};
use std::path::Path;

macro_rules! var_setting {
    ($name:ident, $ty:ty) => {
        #[derive(Config, Debug, Eq, PartialEq)]
        pub struct $name {
            pub default: $ty,
            pub prompt: Option<String>,
            pub required: Option<bool>,
        }
    };
}

var_setting!(TemplateVariableBoolSetting, bool);
var_setting!(TemplateVariableNumberSetting, usize);
var_setting!(TemplateVariableStringSetting, String);

#[derive(Config, Debug, Eq, PartialEq)]
pub struct TemplateVariableEnumValueConfig {
    pub label: String,
    pub value: String,
}

#[derive(Config, Debug, Eq, PartialEq)]
#[config(serde(
    untagged,
    expecting = "expected a value string or value object with label"
))]
pub enum TemplateVariableEnumValue {
    String(String),
    #[setting(nested)]
    Object(TemplateVariableEnumValueConfig),
}

#[derive(Config, Debug, Eq, PartialEq)]
pub struct TemplateVariableEnumSetting {
    pub default: String,
    pub multiple: Option<bool>,
    pub prompt: String,
    #[setting(nested)]
    pub values: Vec<TemplateVariableEnumValue>,
}

#[derive(Config, Debug, Eq, PartialEq)]
#[config(serde(tag = "type", expecting = "expected a supported value type"))]
pub enum TemplateVariable {
    #[setting(nested)]
    Boolean(TemplateVariableBoolSetting),
    #[setting(nested)]
    Enum(TemplateVariableEnumSetting),
    #[setting(nested)]
    Number(TemplateVariableNumberSetting),
    #[setting(nested)]
    String(TemplateVariableStringSetting),
}

/// Docs: https://moonrepo.dev/docs/config/template
#[derive(Config, Debug)]
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

    #[setting(nested)]
    pub variables: FxHashMap<String, TemplateVariable>,
}

impl TemplateConfig {
    pub fn load<P: AsRef<Path>>(path: P) -> miette::Result<TemplateConfig> {
        let result = ConfigLoader::<TemplateConfig>::new()
            .file(path.as_ref())?
            .load()?;

        Ok(result.config)
    }

    pub fn load_from<P: AsRef<Path>>(template_root: P) -> miette::Result<TemplateConfig> {
        Self::load(
            template_root
                .as_ref()
                .join(consts::CONFIG_TEMPLATE_FILENAME),
        )
    }
}
