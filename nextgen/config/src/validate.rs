use schematic::ValidateError;
use semver::Version;

pub fn validate_non_empty(value: &str) -> Result<(), ValidateError> {
    if value.is_empty() {
        return Err(ValidateError::new("must be a non-empty string"));
    }

    Ok(())
}

pub fn validate_semver(value: &str) -> Result<(), ValidateError> {
    if let Err(error) = Version::parse(value) {
        return Err(ValidateError::new(format!(
            "not a valid semantic version: {}",
            error
        )));
    }

    Ok(())
}
