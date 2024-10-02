use moon_action::Operation;
use moon_pdk_api::Operation as PluginOperation;
use moon_process::Command;

pub async fn run_plugin_operation(operation: PluginOperation) -> miette::Result<Operation> {
    match operation {
        PluginOperation::ProcessExecution(process) => {
            Operation::process_execution(&process.command)
                .track_async_with_check(
                    || async {
                        let mut builder = Command::new(process.command);
                        builder.args(process.args);
                        builder.envs(process.env);

                        if let Some(cwd) = process.working_dir.and_then(|cwd| cwd.real_path()) {
                            builder.cwd(cwd);
                        }

                        let mut command = builder.create_async();

                        if process.stream {
                            command.exec_stream_output().await
                        } else {
                            command.exec_capture_output().await
                        }
                    },
                    |result| result.status.success(),
                )
                .await
        }
    }
}
