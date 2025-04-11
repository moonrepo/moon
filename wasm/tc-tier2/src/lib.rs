use extism_pdk::*;
use moon_pdk_api::*;

pub use tc_tier1::*;

#[plugin_fn]
pub fn locate_dependencies_root(
    Json(input): Json<LocateDependenciesRootInput>,
) -> FnResult<Json<LocateDependenciesRootOutput>> {
    Ok(Json(LocateDependenciesRootOutput {
        members: None,
        // We need a root for the `InstallDependencies`
        // action to work, otherwise it aborts early
        root: Some(input.context.workspace_root),
    }))
}

#[plugin_fn]
pub fn install_dependencies(
    Json(_): Json<InstallDependenciesInput>,
) -> FnResult<Json<InstallDependenciesOutput>> {
    Ok(Json(InstallDependenciesOutput::default()))
}
