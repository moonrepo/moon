use crate::validate::{check_list, validate_child_relative_path};
use schematic::{Config, ValidateError};

fn validate_templates(files: &[String]) -> Result<(), ValidateError> {
    if files.is_empty() {
        return Err(ValidateError::new("at least 1 template path is required"));
    }

    check_list(files, |value| validate_child_relative_path(value))?;

    Ok(())
}

#[derive(Config)]
pub struct GeneratorConfig {
    #[setting(
			default = Vec::from(["./templates".into()]),
			validate = validate_templates
		)]
    pub templates: Vec<String>,
}
