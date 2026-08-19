use extism_pdk::*;
use moon_pdk::plugin_err;
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
    Json(input): Json<SetupEnvironmentInput>,
) -> FnResult<Json<SetupEnvironmentOutput>> {
    // Only fail when a test opts in via toolchain config, so that
    // pipeline tests can assert what happens downstream of a hard failure
    if input
        .toolchain_config
        .get("testSetupEnvironmentFailure")
        .and_then(|value| value.as_bool())
        .unwrap_or_default()
    {
        return Err(plugin_err!("Failed to setup environment (test)"));
    }

    Ok(Json(SetupEnvironmentOutput::default()))
}
