use schematic::ValidateError;
use semver::Version;

pub fn validate_semver(value: &str) -> Result<(), ValidateError> {
    if let Err(error) = Version::parse(value) {
        return Err(ValidateError::new(format!(
            "not a valid semantic version: {}",
            error
        )));
    }

    Ok(())
}
