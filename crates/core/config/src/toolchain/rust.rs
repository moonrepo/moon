use crate::validators::is_default;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Clone, Debug, Default, Deserialize, Eq, JsonSchema, PartialEq, Serialize, Validate)]
#[schemars(default)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub struct RustConfig {
    #[serde(skip_serializing_if = "is_default")]
    pub sync_toolchain_config: bool,
}
