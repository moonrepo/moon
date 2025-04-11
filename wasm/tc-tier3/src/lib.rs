use extism_pdk::*;
use moon_pdk_api::*;

pub use tc_tier2::*;

#[plugin_fn]
pub fn setup_toolchain(Json(_): Json<SetupToolchainInput>) -> FnResult<Json<SetupToolchainOutput>> {
    Ok(Json(SetupToolchainOutput::default()))
}
