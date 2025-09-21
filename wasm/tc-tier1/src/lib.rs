use extism_pdk::*;
use moon_pdk_api::*;

#[plugin_fn]
pub fn register_toolchain(
    Json(input): Json<RegisterToolchainInput>,
) -> FnResult<Json<RegisterToolchainOutput>> {
    Ok(Json(RegisterToolchainOutput {
        name: input.id.to_string(),
        ..Default::default()
    }))
}

#[plugin_fn]
pub fn sync_project(Json(input): Json<SyncProjectInput>) -> FnResult<Json<SyncOutput>> {
    let mut output = SyncOutput::default();
    let mut op = Operation::new("sync-project-test")?;

    if input.project.id == "b" {
        if let Some(file) = input.context.workspace_root.join("file.txt").virtual_path() {
            output.changed_files.push(file);
        }
    }

    op.finish(OperationStatus::Failed);

    output.operations.push(op);

    Ok(Json(output))
}

#[plugin_fn]
pub fn sync_workspace(Json(input): Json<SyncWorkspaceInput>) -> FnResult<Json<SyncOutput>> {
    let mut output = SyncOutput::default();
    let mut op = Operation::new("sync-workspace-test")?;

    if let Some(file) = input.context.workspace_root.join("file.txt").virtual_path() {
        output.changed_files.push(file);
    }

    op.finish(OperationStatus::Failed);

    output.operations.push(op);

    Ok(Json(output))
}
