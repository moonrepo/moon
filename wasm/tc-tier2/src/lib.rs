use extism_pdk::*;
use moon_pdk::{VirtualPathExt, get_plugin_id, locate_root};
use moon_pdk_api::*;
use std::fs;

pub use tc_tier1::*;

fn is_testing_deps_workspace(path: &VirtualPath) -> bool {
    let outer = path.to_real_path().unwrap().unwrap();

    // The guest resolves paths with Unix semantics, while real paths from
    // a Windows host contain backslashes, so `file_name` cannot split on
    // them. Extract the last component by handling both separators.
    outer
        .to_string_lossy()
        .rsplit(['/', '\\'])
        .find(|value| !value.is_empty())
        .is_some_and(|value| value == "in" || value == "in-root" || value == "out")
}

#[plugin_fn]
pub fn locate_dependencies_root(
    Json(input): Json<LocateDependenciesRootInput>,
) -> FnResult<Json<LocateDependenciesRootOutput>> {
    record_input(
        &input.context,
        &input.toolchain_config,
        "locate_dependencies_root",
        &input.starting_dir.to_string(),
        json::to_value(&input)?,
    )?;

    Ok(Json(locate_dependencies_root_from(
        &input.context,
        &input.starting_dir,
        &input.toolchain_config,
    )?))
}

/// Locate the root of a starting directory, which `tc-tier2-batch` also
/// calls for each of its starting directories.
pub fn locate_dependencies_root_from(
    context: &MoonContext,
    starting_dir: &VirtualPath,
    toolchain_config: &json::Value,
) -> FnResult<LocateDependenciesRootOutput> {
    // Like a real plugin, locate the root from the project, by traversing
    // upwards to the closest `tc.lock`. When the root has a `tc.root.cfg`,
    // it's a workspace, with a member glob on each line. Tests opt in via
    // toolchain config, as other tests rely on the working dir below.
    if toolchain_config
        .get("testLocateRoots")
        .and_then(|value| value.as_bool())
        .unwrap_or_default()
    {
        let Some(root) = locate_root(starting_dir, "tc.lock") else {
            return Ok(LocateDependenciesRootOutput::default());
        };

        let members_file = root.join("tc.root.cfg");
        let members = if members_file.exists() {
            Some(
                fs::read_to_string(&members_file)?
                    .lines()
                    .map(|line| line.trim())
                    .filter(|line| !line.is_empty())
                    .map(|line| line.to_owned())
                    .collect(),
            )
        } else {
            None
        };

        return Ok(LocateDependenciesRootOutput {
            members,
            root: Some(root),
        });
    }

    // Working dir is set to the project root.
    let cwd = context.working_dir.clone();

    // Testing the `dep-workspace` fixture. The "in" project
    // is in the workspace, while "out" is not.
    let is_deps_workspace = is_testing_deps_workspace(&cwd);

    Ok(LocateDependenciesRootOutput {
        members: if is_deps_workspace {
            Some(vec!["in".into()])
        } else {
            None
        },
        // We need a root for the `InstallDependencies`
        // action to work, otherwise it aborts early
        root: Some(if is_deps_workspace {
            context.workspace_root.clone()
        } else {
            cwd
        }),
    })
}

// Test lockfiles have a `name@version` per line, so that tests can hash
// locked dependencies. Other lines are ignored.
#[plugin_fn]
pub fn parse_lock(Json(input): Json<ParseLockInput>) -> FnResult<Json<ParseLockOutput>> {
    let mut output = ParseLockOutput::default();

    for line in fs::read_to_string(&input.path)?.lines() {
        if let Some((name, version)) = line.trim().split_once('@')
            && !name.is_empty()
        {
            output
                .dependencies
                .entry(name.into())
                .or_default()
                .push(LockDependency {
                    version: Some(VersionSpec::parse(version)?),
                    ..Default::default()
                });
        }
    }

    Ok(Json(output))
}

// Test manifests have a `dep <name> <version>` per line, so that tests can
// hash dependencies. Other lines (like the member globs of `tc.root.cfg`)
// are ignored.
#[plugin_fn]
pub fn parse_manifest(
    Json(input): Json<ParseManifestInput>,
) -> FnResult<Json<ParseManifestOutput>> {
    let mut output = ParseManifestOutput::default();

    for line in fs::read_to_string(&input.path)?.lines() {
        if let ["dep", name, version] = line.split_whitespace().collect::<Vec<_>>().as_slice() {
            output.dependencies.insert(
                (*name).into(),
                ManifestDependency::new(UnresolvedVersionSpec::parse(version)?),
            );
        }
    }

    Ok(Json(output))
}

#[plugin_fn]
pub fn install_dependencies(
    Json(input): Json<InstallDependenciesInput>,
) -> FnResult<Json<InstallDependenciesOutput>> {
    let mut output = InstallDependenciesOutput::default();

    record_input(
        &input.context,
        &input.toolchain_config,
        "install_dependencies",
        &input.root.to_string_lossy(),
        json::to_value(&input)?,
    )?;

    // Write a marker file so that tests can detect if/when this function
    // was called, primarily for pipeline abort/dispatch scenarios
    if input
        .toolchain_config
        .get("testInstallMarker")
        .and_then(|value| value.as_bool())
        .unwrap_or_default()
    {
        let marker = input
            .context
            .workspace_root
            .join(".moon/cache/tcInstallDependencies");

        if let Some(parent) = marker.parent() {
            fs::create_dir_all(&parent)?;
        }

        fs::write(&marker, "")?;
    }

    // Only return commands when a test opts in via toolchain config,
    // otherwise unrelated tests would execute real processes
    if input
        .toolchain_config
        .get("testInstallCommands")
        .and_then(|value| value.as_bool())
        .unwrap_or_default()
    {
        output.install_command =
            Some(ExecCommand::new(ExecCommandInput::pipe("git", ["--version"])).label("install"));
        output.dedupe_command =
            Some(ExecCommand::new(ExecCommandInput::pipe("git", ["--version"])).label("dedupe"));
    }

    Ok(Json(output))
}

#[plugin_fn]
pub fn extend_task_command(
    Json(input): Json<ExtendTaskCommandInput>,
) -> FnResult<Json<ExtendTaskCommandOutput>> {
    let mut output = ExtendTaskCommandOutput::default();

    match input.task.target.get_task_id().unwrap() {
        "command" => {
            output.command = Some("new-command".into());
        }
        "args-empty" => {
            output.args = Some(Extend::Empty);
        }
        "args-append" => {
            output.args = Some(Extend::Append(vec!["new".into(), "arg".into()]));
        }
        "args-prepend" => {
            output.args = Some(Extend::Prepend(vec!["new".into(), "arg".into()]));
        }
        "args-replace" => {
            output.args = Some(Extend::Replace(vec!["new".into(), "arg".into()]));
        }
        "env" => {
            output
                .env
                .insert("EXTENDED_VAR".into(), get_plugin_id()?.to_string());
        }
        "env-remove" => {
            output.env_remove.push("REMOVE_VAR".into());
        }
        "path" => {
            output.paths.push("/extended/path".into());
        }
        "test-ext-and-tc" => {
            output.args = Some(Extend::Append(vec!["from-tc".into()]));
            output.env.insert("FROM_TC".into(), "original".into());
        }
        _ => {}
    };

    Ok(Json(output))
}

#[plugin_fn]
pub fn extend_task_script(
    Json(input): Json<ExtendTaskScriptInput>,
) -> FnResult<Json<ExtendTaskScriptOutput>> {
    let mut output = ExtendTaskScriptOutput::default();

    match input.task.target.get_task_id().unwrap() {
        "env" => {
            output
                .env
                .insert("EXTENDED_VAR".into(), get_plugin_id()?.to_string());
        }
        "env-remove" => {
            output.env_remove.push("REMOVE_VAR".into());
        }
        "path" => {
            output.paths.push("/extended/path".into());
        }
        "script" => {
            output.script = Some(format!("wrapped=$({})", input.script));
        }
        _ => {}
    };

    Ok(Json(output))
}
