use extism_pdk::*;
use moon_pdk_api::*;

pub use tc_tier2::*;

#[plugin_fn]
pub fn define_requirements(
    Json(_): Json<DefineRequirementsInput>,
) -> FnResult<Json<DefineRequirementsOutput>> {
    Ok(Json(DefineRequirementsOutput {
        // Must be tier 3+
        requires: vec!["tc-tier3".into()],
    }))
}
