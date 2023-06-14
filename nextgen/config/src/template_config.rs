// template.yml

use moon_common::consts;
use rustc_hash::FxHashMap;
use schematic::{
    derive_enum, validate, Config, ConfigError, ConfigLoader, SchemaField, SchemaType, Schematic,
};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TemplateVariableSetting<T> {
    pub default: T,
    pub prompt: Option<String>,
    pub required: Option<bool>,
}

impl<T: Schematic> Schematic for TemplateVariableSetting<T> {
    fn generate_schema() -> SchemaType {
        SchemaType::structure(vec![
            SchemaField::new("default", SchemaType::infer::<T>()),
            SchemaField::new("prompt", SchemaType::infer::<Option<String>>()),
            SchemaField::new("required", SchemaType::infer::<Option<bool>>()),
        ])
    }
}

derive_enum!(
    #[serde(
        untagged,
        expecting = "expected a value string or value object with label"
    )]
    pub enum TemplateVariableEnumValue {
        String(String),
        Object { label: String, value: String },
    }
);

impl Schematic for TemplateVariableEnumValue {
    fn generate_schema() -> SchemaType {
        let mut schema = SchemaType::union(vec![
            SchemaType::string(),
            SchemaType::structure(vec![
                SchemaField::new("label", SchemaType::string()),
                SchemaField::new("value", SchemaType::string()),
            ]),
        ]);
        schema.set_name("TemplateVariableEnumValue");
        schema
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TemplateVariableEnumSetting {
    pub default: String,
    pub multiple: Option<bool>,
    pub prompt: String,
    pub values: Vec<TemplateVariableEnumValue>,
}

impl Schematic for TemplateVariableEnumSetting {
    fn generate_schema() -> SchemaType {
        let mut schema = SchemaType::structure(vec![
            SchemaField::new("default", SchemaType::string()),
            SchemaField::new("multiple", SchemaType::infer::<Option<bool>>()),
            SchemaField::new("prompt", SchemaType::string()),
            SchemaField::new(
                "values",
                SchemaType::infer::<Vec<TemplateVariableEnumValue>>(),
            ),
        ]);
        schema.set_name("TemplateVariableEnumSetting");
        schema
    }
}

derive_enum!(
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

impl Schematic for TemplateVariable {
    fn generate_schema() -> SchemaType {
        let add_type = |schema: &mut SchemaType| {
            if let SchemaType::Struct(inner) = schema {
                inner
                    .fields
                    .push(SchemaField::new("type", SchemaType::string()));
            }
        };

        let mut b = TemplateVariableSetting::<bool>::generate_schema();
        add_type(&mut b);

        // if let SchemaType::Struct(b) = &mut b {
        //     b.fields.push(SchemaField::new(
        //         "type",
        //         SchemaType::literal(LiteralValue::String("boolean".into())),
        //     ));
        // }

        let mut e = TemplateVariableEnumSetting::generate_schema();
        add_type(&mut e);

        // if let SchemaType::Struct(e) = &mut e {
        //     e.fields.push(SchemaField::new(
        //         "type",
        //         SchemaType::literal(LiteralValue::String("enum".into())),
        //     ));
        // }

        let mut n = TemplateVariableSetting::<usize>::generate_schema();
        add_type(&mut n);

        // if let SchemaType::Struct(n) = &mut n {
        //     n.fields.push(SchemaField::new(
        //         "type",
        //         SchemaType::literal(LiteralValue::String("number".into())),
        //     ));
        // }

        let mut s = TemplateVariableSetting::<String>::generate_schema();
        add_type(&mut s);

        // if let SchemaType::Struct(s) = &mut s {
        //     s.fields.push(SchemaField::new(
        //         "type",
        //         SchemaType::literal(LiteralValue::String("string".into())),
        //     ));
        // }

        let mut schema = SchemaType::union(vec![b, e, n, s]);
        schema.set_name("TemplateVariable");
        schema
    }
}

/// Docs: https://moonrepo.dev/docs/config/template
#[derive(Debug, Config)]
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
    pub fn load<P: AsRef<Path>>(path: P) -> Result<TemplateConfig, ConfigError> {
        let result = ConfigLoader::<TemplateConfig>::new()
            .file(path.as_ref())?
            .load()?;

        Ok(result.config)
    }

    pub fn load_from<P: AsRef<Path>>(template_root: P) -> Result<TemplateConfig, ConfigError> {
        Self::load(
            template_root
                .as_ref()
                .join(consts::CONFIG_TEMPLATE_FILENAME),
        )
    }
}
