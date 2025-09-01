use extism_pdk::*;
use moon_pdk_api::*;
// use std::path::PathBuf;

pub use tc_tier2::*;

#[plugin_fn]
pub fn setup_toolchain(Json(_): Json<SetupToolchainInput>) -> FnResult<Json<SetupToolchainOutput>> {
    Ok(Json(SetupToolchainOutput::default()))
}

// #[plugin_fn]
// pub fn register_tool(Json(input): Json<RegisterToolInput>) -> FnResult<Json<RegisterToolOutput>> {
//     Ok(Json(RegisterToolOutput {
//         name: input.id.into(),
//         ..Default::default()
//     }))
// }

// #[plugin_fn]
// pub fn load_versions(Json(_): Json<LoadVersionsInput>) -> FnResult<Json<LoadVersionsOutput>> {
//     Ok(Json(LoadVersionsOutput {
//         versions: vec![
//             VersionSpec::parse("1.2.3").unwrap(),
//             VersionSpec::parse("4.5.6").unwrap(),
//         ],
//         ..Default::default()
//     }))
// }

// #[plugin_fn]
// pub fn locate_executables(
//     Json(_): Json<LocateExecutablesInput>,
// ) -> FnResult<Json<LocateExecutablesOutput>> {
//     Ok(Json(LocateExecutablesOutput {
//         exes_dirs: vec![PathBuf::from("bin")],
//         ..Default::default()
//     }))
// }
