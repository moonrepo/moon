use schematic::ValidateError;
use std::path::Path;

#[cfg(feature = "loader")]
pub fn check_yml_extension(path: &Path) -> std::path::PathBuf {
    let mut yaml_path = path.to_path_buf();
    yaml_path.set_extension("yaml");

    if yaml_path.exists() {
        #[cfg(feature = "tracing")]
        {
            use moon_common::color;

            tracing::warn!(
                config = ?yaml_path,
                "Found a config file with the {} extension, please use {} instead. We'll continue to load the file, but this will cause unintended side-effects!",
                color::file(".yaml"),
                color::file(".yml"),
            );
        }

        return yaml_path;
    }

    path.to_path_buf()
}

pub fn validate_relative_path(value: &str) -> Result<(), ValidateError> {
    let path = Path::new(value);

    if path.has_root() || path.is_absolute() {
        return Err(ValidateError::new("absolute paths are not supported"));
    }

    Ok(())
}

pub fn validate_child_relative_path(value: &str) -> Result<(), ValidateError> {
    if value.contains("..") {
        return Err(ValidateError::new(
            "parent directory traversal (..) is not supported",
        ));
    }

    Ok(())
}
