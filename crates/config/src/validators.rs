use semver::Version;
use validator::ValidationError;

pub fn validate_version(value: &str) -> Result<(), ValidationError> {
    if Version::parse(value).is_err() {
        return Err(ValidationError::new("version_invalid_semver"));
    }

    Ok(())
}
