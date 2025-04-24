use moon_pdk_api::ExecCommandInput;
use moon_process::Command;
use std::process::Output;

pub async fn exec_plugin_command(input: &ExecCommandInput) -> miette::Result<Output> {
    let mut command = Command::new(&input.command);
    command.args(&input.args);
    command.envs(&input.env);

    if let Some(cwd) = input.working_dir.as_ref().and_then(|dir| dir.real_path()) {
        command.cwd(cwd);
    }

    let output = if input.stream {
        command.exec_stream_output().await?
    } else {
        command.exec_capture_output().await?
    };

    Ok(output)
}
