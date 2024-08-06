mod common;
mod extension;
mod toolchain;

pub use common::*;
pub use extension::*;
pub use toolchain::*;
pub use warpgate_api::*;

/// Apply default attributes for configuration based structs.
/// Will assume that all keys are in camel case.
#[macro_export]
macro_rules! config_struct {
    ($struct:item) => {
        #[derive(Debug, serde::Deserialize)]
        #[serde(default, deny_unknown_fields, rename_all = "camelCase")]
        $struct
    };
}
