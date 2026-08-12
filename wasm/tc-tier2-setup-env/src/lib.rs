use extism_pdk::*;
use moon_pdk_api::*;

pub use tc_tier2::*;

#[plugin_fn]
pub fn define_requirements(
    Json(input): Json<DefineRequirementsInput>,
) -> FnResult<Json<DefineRequirementsOutput>> {
    let mut output = DefineRequirementsOutput::default();

    // Only require other toolchains when a test opts in via toolchain config,
    // otherwise unrelated tests would gain unexpected actions and edges
    if let Some(requires) = input
        .toolchain_config
        .get("testEnvRequirements")
        .and_then(|value| value.as_array())
    {
        output.requires = requires
            .iter()
            .filter_map(|value| value.as_str().map(|id| id.to_owned()))
            .collect();
        output.for_setup_environment = true;
    }

    Ok(Json(output))
}

#[plugin_fn]
pub fn setup_environment(
    Json(_): Json<SetupEnvironmentInput>,
) -> FnResult<Json<SetupEnvironmentOutput>> {
    Ok(Json(SetupEnvironmentOutput::default()))
}
