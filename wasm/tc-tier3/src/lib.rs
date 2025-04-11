use extism_pdk::*;
use moon_pdk_api::*;

#[plugin_fn]
pub fn register_toolchain(
    Json(_): Json<RegisterToolchainInput>,
) -> FnResult<Json<RegisterToolchainOutput>> {
    Ok(Json(RegisterToolchainOutput {
        name: "tc-tier3".into(),
        ..Default::default()
    }))
}

// TIER 3

#[plugin_fn]
pub fn setup_toolchain(Json(_): Json<SetupToolchainInput>) -> FnResult<Json<SetupToolchainOutput>> {
    Ok(Json(SetupToolchainOutput::default()))
}
