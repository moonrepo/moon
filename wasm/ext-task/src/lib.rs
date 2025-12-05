use extism_pdk::*;
use moon_pdk::get_plugin_id;
use moon_pdk_api::*;

#[plugin_fn]
pub fn register_extension(
    Json(input): Json<RegisterExtensionInput>,
) -> FnResult<Json<RegisterExtensionOutput>> {
    Ok(Json(RegisterExtensionOutput {
        name: input.id.to_string(),
        ..Default::default()
    }))
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
        "test-ext-and-tc" => {
            if input.args.iter().any(|arg| arg == "from-tc") {
                output.args = Some(Extend::Prepend(vec!["from-ext".into()]));
            }

            output.env.insert("FROM_TC".into(), "overwritten".into());
            output.env.insert("FROM_EXT".into(), "original".into());
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
