use crate::errors::create_validation_error;
use semver::Version;
use std::path::Path;
use validator::ValidationError;

// Validate the value is a valid semver version/range.
pub fn validate_semver_version(key: &str, value: &str) -> Result<(), ValidationError> {
    if Version::parse(value).is_err() {
        return Err(create_validation_error(
            "invalid_semver",
            key,
            String::from("Must be a valid semantic version."),
        ));
    }

    Ok(())
}

// Validate the value is a valid child relative file system path.
// Will fail on absolute paths ("/"), and parent relative paths ("../").
pub fn validate_child_relative_path(key: &str, value: &String) -> Result<(), ValidationError> {
    let path = Path::new(value);

    if path.has_root() || path.is_absolute() {
        return Err(create_validation_error(
            "no_absolute",
            key,
            String::from("Absolute paths are not supported."),
        ));
    } else if path.starts_with("..") {
        return Err(create_validation_error(
            "no_parent_relative",
            key,
            String::from("Parent relative paths are not supported."),
        ));
    }

    Ok(())
}
