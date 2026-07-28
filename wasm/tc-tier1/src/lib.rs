use extism_pdk::*;
use moon_pdk::get_plugin_id;
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
pub fn extend_project_graph(
    Json(input): Json<ExtendProjectGraphInput>,
) -> FnResult<Json<ExtendProjectGraphOutput>> {
    let mut output = ExtendProjectGraphOutput::default();

    // This function is exported by all `tc-tier*` plugins through crate
    // re-exports, but only one should act, otherwise the concurrent
    // instances race on the same marker and manifest files
    if get_plugin_id()? != "tc-tier1" {
        return Ok(Json(output));
    }

    // Write a marker file so that tests can detect if/when this
    // function was called, primarily for caching scenarios
    let marker = input
        .context
        .workspace_root
        .join(".moon/cache/tcExtendProjectGraph");

    if let Some(parent) = marker.parent() {
        fs::create_dir_all(&parent)?;
    }

    fs::write(&marker, "")?;

    // Extract an alias from the manifest declared in our metadata
    for (id, source) in &input.project_sources {
        let manifest = input.context.workspace_root.join(source).join("tc.cfg");

        if manifest.exists() {
            let alias = fs::read_to_string(&manifest)?.trim().to_owned();

            if !alias.is_empty() {
                output.extended_projects.insert(
                    id.to_owned(),
                    ExtendProjectOutput {
                        alias: Some(alias),
                        ..Default::default()
                    },
                );
            }

            if let Some(file) = manifest.virtual_path() {
                output.input_files.push(file);
            }
        }
    }

    Ok(Json(output))
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

#[plugin_fn]
pub fn prune_docker(Json(input): Json<PruneDockerInput>) -> FnResult<Json<PruneDockerOutput>> {
    let mut output = PruneDockerOutput::default();

    for project in ["dep", "scaffold", "prune"] {
        let vendor_dir = input.root.join(project).join("vendor");

        if vendor_dir.exists() && input.docker_config.delete_vendor_directories {
            fs::remove_dir_all(&vendor_dir)?;

            if let Some(file) = vendor_dir.virtual_path() {
                output.changed_files.push(file);
            }
        }
    }

    Ok(Json(output))
}
