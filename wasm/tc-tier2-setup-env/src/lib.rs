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
    // otherwise unrelated tests would gain unexpected actions and edges.
    // Requirements whose environments are setup before this toolchain's, like
    // a parent toolchain (JavaScript) that locates the dependencies root for its
    // children, or whose toolchains are setup before it when they don't setup
    // an environment
    if let Some(ids) = get_ids("testEnvRequirements").or_else(|| get_ids("testRequirements")) {
        output.requires = ids
            .into_iter()
            .map(|id| ToolchainRequirement {
                id,
                setup_toolchain: None,
                setup_environment: Some(RequirementPhase::Before),
            })
            .collect();
    }
    // Requirements with any phases
    else if let Some(requires) = input.toolchain_config.get("testRequires") {
        output.requires = json::from_value(requires.to_owned())?;
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
