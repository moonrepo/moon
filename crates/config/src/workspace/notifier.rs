use crate::validators::validate_url;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use validator::{Validate, ValidationError};

fn validate_webhook_url(url: &str) -> Result<(), ValidationError> {
    validate_url("webhookUrl", url, true)?;

    Ok(())
}

#[derive(Clone, Debug, Default, Deserialize, Eq, JsonSchema, PartialEq, Serialize, Validate)]
#[schemars(default)]
#[serde(rename_all = "camelCase")]
pub struct NotifierConfig {
    #[validate(custom = "validate_webhook_url")]
    pub webhook_url: Option<String>,
}
