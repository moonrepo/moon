use crate::validators::is_default;
use crate::{
    errors::create_validation_error, types::FilePath, validators::validate_child_relative_path,
};
use moon_utils::string_vec;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use validator::{Validate, ValidationError};

fn validate_templates(files: &[FilePath]) -> Result<(), ValidationError> {
    if files.is_empty() {
        return Err(create_validation_error(
            "no_templates",
            "templates",
            "At least 1 template path is required",
        ));
    }

    for (index, file) in files.iter().enumerate() {
        validate_child_relative_path(format!("templates[{index}]"), file)?;
    }

    Ok(())
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, Validate)]
#[schemars(default)]
#[serde(default, rename_all = "camelCase")]
pub struct GeneratorConfig {
    #[serde(skip_serializing_if = "is_default")]
    #[validate(custom = "validate_templates")]
    pub templates: Vec<FilePath>,
}

impl Default for GeneratorConfig {
    fn default() -> Self {
        GeneratorConfig {
            templates: string_vec!["./templates"],
        }
    }
}
