use extism_pdk::*;
use moon_pdk_api::*;

pub use tc_tier1::*;

fn is_testing_deps_workspace(path: &VirtualPath) -> bool {
    let outer = match path {
        VirtualPath::OnlyReal(inner) => inner,
        // Don't use `path` since it gets replaced with the virtual
        // path, which masks the folder we're actually in on the host
        VirtualPath::WithReal {
            real_prefix: inner, ..
        } => inner,
    };

    outer.ends_with("in") || outer.ends_with("in-root") || outer.ends_with("out")
}

#[plugin_fn]
pub fn locate_dependencies_root(
    Json(input): Json<LocateDependenciesRootInput>,
) -> FnResult<Json<LocateDependenciesRootOutput>> {
    // Working dir is set to the project root.
    let cwd = input.context.working_dir;

    // Testing the `dep-workspace` fixture. The "in" project
    // is in the workspace, while "out" is not.
    let is_deps_workspace = is_testing_deps_workspace(&cwd);

    Ok(Json(LocateDependenciesRootOutput {
        members: if is_deps_workspace {
            Some(vec!["in".into()])
        } else {
            None
        },
        // We need a root for the `InstallDependencies`
        // action to work, otherwise it aborts early
        root: Some(if is_deps_workspace {
            input.context.workspace_root
        } else {
            cwd
        }),
    }))
}

#[plugin_fn]
pub fn install_dependencies(
    Json(_): Json<InstallDependenciesInput>,
) -> FnResult<Json<InstallDependenciesOutput>> {
    Ok(Json(InstallDependenciesOutput::default()))
}
