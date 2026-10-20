// Like `tc-tier2`, but also locates the roots of many directories in a single
// call, so that tests can compare it with locating one root per call
use extism_pdk::*;
use moon_pdk_api::*;

pub use tc_tier2::*;

#[plugin_fn]
pub fn locate_dependencies_roots(
    Json(input): Json<LocateDependenciesRootsInput>,
) -> FnResult<Json<LocateDependenciesRootsOutput>> {
    // Keyed by the first directory, as each directory is in one call
    record_input(
        &input.context,
        &input.toolchain_config,
        "locate_dependencies_roots",
        &input
            .starting_dirs
            .first()
            .map(|dir| dir.to_string())
            .unwrap_or_default(),
        json::to_value(&input)?,
    )?;

    let mut output = LocateDependenciesRootsOutput::default();

    for starting_dir in &input.starting_dirs {
        output.roots.push(locate_dependencies_root_from(
            &input.context,
            starting_dir,
            &input.toolchain_config,
        )?);
    }

    // Return a root too few, so that tests can check that it's validated
    if input
        .toolchain_config
        .get("testMismatchRoots")
        .and_then(|value| value.as_bool())
        .unwrap_or_default()
    {
        output.roots.pop();
    }

    Ok(Json(output))
}
