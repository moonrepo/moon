use extism_pdk::*;
use moon_pdk_api::*;

pub use tc_tier2::*;

#[plugin_fn]
pub fn setup_environment(
    Json(_): Json<SetupEnvironmentInput>,
) -> FnResult<Json<SetupEnvironmentOutput>> {
    Ok(Json(SetupEnvironmentOutput::default()))
}
