use extism_pdk::*;
use moon_pdk::exec_process_command;
use moon_pdk_api::*;

#[host_fn]
extern "ExtismHost" {
    fn exec_process_command_v1(input: Json<ProcessCommandInput>) -> Json<ProcessCommandResult>;
    fn read_process_output_v1(input: Json<ProcessOutputChunkInput>) -> Json<ProcessOutputChunk>;
    fn close_process_output_v1(
        input: Json<ProcessOutputCloseInput>,
    ) -> Json<ProcessOutputCloseOutput>;
}

#[plugin_fn]
pub fn register_vcs(Json(_): Json<RegisterVcsInput>) -> FnResult<Json<RegisterVcsOutput>> {
    Ok(Json(RegisterVcsOutput {
        name: "Process host fixture".into(),
        plugin_version: "0.0.1".into(),
        protocol_version: VCS_PLUGIN_PROTOCOL_VERSION,
        process_capabilities: vec![ProcessCapabilityDeclaration {
            id: Id::raw("git"),
            executable: "git".into(),
        }],
        ..Default::default()
    }))
}

#[plugin_fn]
pub fn execute_process(
    Json(input): Json<ProcessCommandInput>,
) -> FnResult<Json<(i32, Vec<u8>, Vec<u8>)>> {
    let output = exec_process_command(input)?;

    Ok(Json((output.exit_code, output.stdout, output.stderr)))
}

#[plugin_fn]
pub fn start_process(
    Json(input): Json<ProcessCommandInput>,
) -> FnResult<Json<ProcessCommandResult>> {
    Ok(unsafe { exec_process_command_v1(Json(input))? })
}

#[plugin_fn]
pub fn read_process_output(
    Json(input): Json<ProcessOutputChunkInput>,
) -> FnResult<Json<ProcessOutputChunk>> {
    Ok(unsafe { read_process_output_v1(Json(input))? })
}

#[plugin_fn]
pub fn close_process_output(
    Json(input): Json<ProcessOutputCloseInput>,
) -> FnResult<Json<ProcessOutputCloseOutput>> {
    Ok(unsafe { close_process_output_v1(Json(input))? })
}
