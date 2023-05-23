// use moon_common::is_test_env;
use schematic::{validate, Config, ValidateError};

fn validate_webhook_url<T: AsRef<str>, D, C>(
    url: T,
    data: &D,
    ctx: &C,
) -> Result<(), ValidateError> {
    // if !is_test_env() {
    validate::url_secure(&url, data, ctx)?;
    // }

    Ok(())
}

#[derive(Config)]
pub struct NotifierConfig {
    #[setting(validate = validate_webhook_url)]
    pub webhook_url: Option<String>,
}
