use extism_pdk::*;
use moon_pdk_api::*;

#[plugin_fn]
pub fn register_toolchain(
    Json(_): Json<RegisterToolchainInput>,
) -> FnResult<Json<RegisterToolchainOutput>> {
    Ok(Json(RegisterToolchainOutput {
        name: "tc-tier1".into(),
        ..Default::default()
    }))
}
