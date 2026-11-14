use extism_pdk::*;
use moon_pdk_api::*;

pub use tc_tier2::*;

#[plugin_fn]
pub fn define_requirements(
    Json(input): Json<DefineRequirementsInput>,
) -> FnResult<Json<DefineRequirementsOutput>> {
    Ok(Json(DefineRequirementsOutput {
        // Must be tier 3+
        requires: vec![ToolchainRequirement {
            id: "tc-tier3".into(),
            setup_toolchain: Some(RequirementPhase::Before),
            // Only require for the environment when a test opts in via toolchain
            // config, as this plugin does not implement `setup_environment` and
            // is used to verify that an anchor action is still created
            setup_environment: input
                .toolchain_config
                .get("testRequiresForEnvironment")
                .and_then(|value| value.as_bool())
                .unwrap_or_default()
                .then_some(RequirementPhase::Before),
        }],
    }))
}
