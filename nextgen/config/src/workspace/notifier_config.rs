use moon_common::is_test_env;
use schematic::{validate, Config, ValidateError};

fn validate_webhook_url<T: AsRef<str>>(url: T) -> Result<(), ValidateError> {
    if !is_test_env() {
        validate::url(&url)?;
    }

    Ok(())
}

#[derive(Config)]
pub struct NotifierConfig {
    #[setting(validate = validate_webhook_url)]
    pub webhook_url: Option<String>,
}
