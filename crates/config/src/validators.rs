use semver::Version;
use validator::ValidationError;

pub fn validate_version(value: &str) -> Result<(), ValidationError> {
	if let Err(_) = Version::parse(value) {
		return Err(ValidationError::new("version_invalid_semver"));
	}

	Ok(())
}
