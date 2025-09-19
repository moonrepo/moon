use extism_pdk::*;
use moon_pdk::get_plugin_id;
use moon_pdk_api::*;

pub use tc_tier1::*;

fn is_testing_deps_workspace(path: &VirtualPath) -> bool {
    let outer = match path {
        VirtualPath::Real(inner) => inner,
        // Don't use `path` since it gets replaced with the virtual
        // path, which masks the folder we're actually in on the host
        VirtualPath::Virtual {
            real_prefix: inner, ..
        } => inner,
    };

    // // `ends_with` or `file_name` didn't work on Windows...
    // let value = outer.to_string_lossy();
    // let res = value.ends_with("in") || value.ends_with("in-root") || value.ends_with("out");

    // res

    outer
        .file_name()
        .and_then(|file| file.to_str())
        .is_some_and(|value| value == "in" || value == "in-root" || value == "out")
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
        root: if is_deps_workspace {
            input.context.workspace_root.virtual_path()
        } else {
            cwd.virtual_path()
        },
    }))
}

#[plugin_fn]
pub fn install_dependencies(
    Json(_): Json<InstallDependenciesInput>,
) -> FnResult<Json<InstallDependenciesOutput>> {
    Ok(Json(InstallDependenciesOutput::default()))
}

#[plugin_fn]
pub fn extend_task_command(
    Json(input): Json<ExtendTaskCommandInput>,
) -> FnResult<Json<ExtendTaskCommandOutput>> {
    let mut output = ExtendTaskCommandOutput::default();

    match input.task.target.task_id.as_str() {
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
        _ => {}
    };

    Ok(Json(output))
}

#[plugin_fn]
pub fn extend_task_script(
    Json(input): Json<ExtendTaskScriptInput>,
) -> FnResult<Json<ExtendTaskScriptOutput>> {
    let mut output = ExtendTaskScriptOutput::default();

    match input.task.target.task_id.as_str() {
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
