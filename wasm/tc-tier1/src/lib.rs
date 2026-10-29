use extism_pdk::*;
use moon_pdk::get_plugin_id;
use moon_pdk_api::*;
use std::fs;

/// When a test opts in via toolchain config (`testRecordInputs`), record the
/// input of a function call, at `.moon/cache/tcInputs/<plugin>/<func>/<key>.json`
/// in the workspace, so that tests can assert what moon passes to plugins.
pub fn record_input(
    context: &MoonContext,
    toolchain_config: &json::Value,
    func: &str,
    key: &str,
    input: json::Value,
) -> FnResult<()> {
    if !toolchain_config
        .get("testRecordInputs")
        .and_then(|value| value.as_bool())
        .unwrap_or_default()
    {
        return Ok(());
    }

    let key = key
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '.' || ch == '-' {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>();

    let file = context
        .workspace_root
        .join(".moon/cache/tcInputs")
        .join(get_plugin_id()?.as_str())
        .join(func)
        .join(format!("{key}.json"));

    if let Some(parent) = file.parent() {
        fs::create_dir_all(&parent)?;
    }

    fs::write(&file, json::to_string_pretty(&input)?)?;

    Ok(())
}

#[plugin_fn]
pub fn register_toolchain(
    Json(input): Json<RegisterToolchainInput>,
) -> FnResult<Json<RegisterToolchainOutput>> {
    Ok(Json(RegisterToolchainOutput {
        name: input.id.to_string(),
        manifest_file_names: vec!["tc.cfg".into(), "tc.root.cfg".into()],
        lock_file_names: vec!["tc.lock".into()],
        vendor_dir_name: Some("vendor".into()),
        // Opt out, so that tests can verify toolchains that don't inject them
        inject_task_inputs: input.id != "tc-tier2-setup-env",
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

            output.input_files.push(manifest);
        }

        // Lock files are not registered as manifests, so they're excluded
        // from the graph cache's invalidation heuristic. Report them as
        // input files so that tests can verify the discovered-files flow
        let lockfile = input.context.workspace_root.join(source).join("tc.lock");

        if lockfile.exists() {
            output.input_files.push(lockfile);
        }
    }

    Ok(Json(output))
}

#[plugin_fn]
pub fn sync_project(Json(input): Json<SyncProjectInput>) -> FnResult<Json<SyncOutput>> {
    let mut output = SyncOutput::default();
    let mut op = Operation::new("sync-project-test")?;

    if input.project.id == "b" {
        output
            .changed_files
            .push(input.context.workspace_root.join("file.txt"));
    }

    op.finish(OperationStatus::Failed);

    output.operations.push(op);

    Ok(Json(output))
}

#[plugin_fn]
pub fn sync_workspace(Json(input): Json<SyncWorkspaceInput>) -> FnResult<Json<SyncOutput>> {
    let mut output = SyncOutput::default();
    let mut op = Operation::new("sync-workspace-test")?;

    output
        .changed_files
        .push(input.context.workspace_root.join("file.txt"));

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
            output.copied_files.push(path);
        }
        ScaffoldDockerPhase::Sources => {
            let path = input.output_dir.join("from-sources-phase");
            fs::write(&path, "")?;
            output.copied_files.push(path);
        }
    };

    Ok(Json(output))
}

#[plugin_fn]
pub fn prune_docker(Json(input): Json<PruneDockerInput>) -> FnResult<Json<PruneDockerOutput>> {
    let mut output = PruneDockerOutput::default();

    record_input(
        &input.context,
        &input.toolchain_config,
        "prune_docker",
        &input.root.to_string_lossy(),
        json::to_value(&input)?,
    )?;

    for project in ["dep", "scaffold", "prune"] {
        let vendor_dir = input.root.join(project).join("vendor");

        if vendor_dir.exists() && input.docker_config.delete_vendor_directories {
            fs::remove_dir_all(&vendor_dir)?;

            output.changed_files.push(vendor_dir);
        }
    }

    Ok(Json(output))
}
