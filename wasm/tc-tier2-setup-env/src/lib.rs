use extism_pdk::*;
use moon_pdk::plugin_err;
use moon_pdk_api::*;

pub use tc_tier2::*;

#[plugin_fn]
pub fn define_requirements(
    Json(input): Json<DefineRequirementsInput>,
) -> FnResult<Json<DefineRequirementsOutput>> {
    let mut output = DefineRequirementsOutput::default();

    let get_ids = |key: &str| {
        input
            .toolchain_config
            .get(key)
            .and_then(|value| value.as_array())
            .map(|ids| {
                ids.iter()
                    .filter_map(|value| value.as_str().map(|id| id.to_owned()))
                    .collect::<Vec<_>>()
            })
    };

    // Only require other toolchains when a test opts in via toolchain config,
    // otherwise unrelated tests would gain unexpected actions and edges
    if let Some(requires) = get_ids("testEnvRequirements") {
        output.requires = requires;
        output.for_setup_environment = true;
    }
    // Requirements that don't apply to any action, like a parent toolchain
    // (JavaScript) that locates the dependencies root for its children
    else if let Some(requires) = get_ids("testRequirements") {
        output.requires = requires;
    }

    Ok(Json(output))
}

#[plugin_fn]
pub fn setup_environment(
    Json(input): Json<SetupEnvironmentInput>,
) -> FnResult<Json<SetupEnvironmentOutput>> {
    record_input(
        &input.context,
        &input.toolchain_config,
        "setup_environment",
        &input.root.to_string_lossy(),
        json::to_value(&input)?,
    )?;

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
