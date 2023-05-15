use crate::validate::validate_child_relative_path;
use schematic::{Config, Segment, ValidateError};

fn validate_templates(files: &[String]) -> Result<(), ValidateError> {
    if files.is_empty() {
        return Err(ValidateError::new("at least 1 template path is required"));
    }

    for (i, file) in files.iter().enumerate() {
        validate_child_relative_path(file).map_err(|error| {
            ValidateError::with_segments(error.message, vec![Segment::Index(i)])
        })?;
    }

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
