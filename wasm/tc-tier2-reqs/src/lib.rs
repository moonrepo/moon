use extism_pdk::*;
use moon_pdk_api::*;

pub use tc_tier2::*;

#[plugin_fn]
pub fn define_requirements(
    Json(input): Json<DefineRequirementsInput>,
) -> FnResult<Json<DefineRequirementsOutput>> {
    Ok(Json(DefineRequirementsOutput {
        // Must be tier 3+
        requires: vec!["tc-tier3".into()],
        for_setup_toolchain: true,
        // Only require for the environment when a test opts in via toolchain
        // config, as this plugin does not implement `setup_environment` and
        // is used to verify that an anchor action is still created
        for_setup_environment: input
            .toolchain_config
            .get("testRequiresForEnvironment")
            .and_then(|value| value.as_bool())
            .unwrap_or_default(),
    }))
}
