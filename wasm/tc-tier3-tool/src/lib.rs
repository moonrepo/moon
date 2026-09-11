use extism_pdk::*;
use moon_pdk_api::*;
use proto_pdk_api::{
    ExecutableConfig, LoadVersionsInput, LoadVersionsOutput, LocateExecutablesInput,
    LocateExecutablesOutput, RegisterToolInput, RegisterToolOutput, VersionSpec,
};
use rustc_hash::FxHashMap;
use std::path::PathBuf;

// Unlike the other `tc-tier3*` test plugins, this one registers a real proto
// tool, so that the install/locate flows (and the failures they produce when
// a toolchain was never setup) can be exercised in tests.

#[plugin_fn]
pub fn register_toolchain(
    Json(input): Json<RegisterToolchainInput>,
) -> FnResult<Json<RegisterToolchainOutput>> {
    Ok(Json(RegisterToolchainOutput {
        name: input.id.to_string(),
        exe_names: vec!["tc-tool".into()],
        ..Default::default()
    }))
}

#[plugin_fn]
pub fn register_tool(Json(input): Json<RegisterToolInput>) -> FnResult<Json<RegisterToolOutput>> {
    Ok(Json(RegisterToolOutput {
        name: input.id.to_string(),
        ..Default::default()
    }))
}

#[plugin_fn]
pub fn load_versions(Json(_): Json<LoadVersionsInput>) -> FnResult<Json<LoadVersionsOutput>> {
    Ok(Json(LoadVersionsOutput {
        versions: vec![VersionSpec::parse("1.2.3").unwrap()],
        ..Default::default()
    }))
}

#[plugin_fn]
pub fn locate_executables(
    Json(_): Json<LocateExecutablesInput>,
) -> FnResult<Json<LocateExecutablesOutput>> {
    Ok(Json(LocateExecutablesOutput {
        exes: FxHashMap::from_iter([(
            "tc-tool".into(),
            ExecutableConfig {
                exe_path: Some(PathBuf::from("bin").join("tc-tool")),
                primary: true,
                ..Default::default()
            },
        )]),
        ..Default::default()
    }))
}
