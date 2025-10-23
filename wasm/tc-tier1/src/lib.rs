use extism_pdk::*;
use moon_pdk_api::*;
use std::fs;

#[plugin_fn]
pub fn register_toolchain(
    Json(input): Json<RegisterToolchainInput>,
) -> FnResult<Json<RegisterToolchainOutput>> {
    Ok(Json(RegisterToolchainOutput {
        name: input.id.to_string(),
        manifest_file_names: vec!["tc.cfg".into(), "tc.root.cfg".into()],
        lock_file_names: vec!["tc.lock".into()],
        vendor_dir_name: Some("vendor".into()),
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

#[plugin_fn]
pub fn scaffold_docker(
    Json(input): Json<ScaffoldDockerInput>,
) -> FnResult<Json<ScaffoldDockerOutput>> {
    let mut output = ScaffoldDockerOutput::default();

    match input.phase {
        ScaffoldDockerPhase::Configs => {
            let path = input.output_dir.join("from-configs-phase");
            fs::write(&path, "")?;
            output.copied_files.push(path.virtual_path().unwrap());
        }
        ScaffoldDockerPhase::Sources => {
            let path = input.output_dir.join("from-sources-phase");
            fs::write(&path, "")?;
            output.copied_files.push(path.virtual_path().unwrap());
        }
    };

    Ok(Json(output))
}
