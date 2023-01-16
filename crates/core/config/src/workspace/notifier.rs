use crate::validators::validate_url;
use moon_utils::is_test_env;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use validator::{Validate, ValidationError};

fn validate_webhook_url(url: &str) -> Result<(), ValidationError> {
    validate_url("webhookUrl", url, !is_test_env())?;

    Ok(())
}

#[derive(Clone, Debug, Default, Deserialize, Eq, JsonSchema, PartialEq, Serialize, Validate)]
#[schemars(default)]
#[serde(default, rename_all = "camelCase")]
pub struct NotifierConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(custom = "validate_webhook_url")]
    pub webhook_url: Option<String>,
}
