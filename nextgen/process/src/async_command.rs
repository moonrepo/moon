use crate::command_inspector::CommandInspector;
use crate::output_to_error;
use crate::process_error::ProcessError;
use std::process::{ExitStatus, Output, Stdio};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};

pub struct AsyncCommand<'cmd> {
    inner: Command,
    inspector: CommandInspector<'cmd>,
}

impl<'cmd> AsyncCommand<'cmd> {
    pub async fn exec_capture_output(&mut self) -> Result<Output, ProcessError> {
        // self.log_command_info();

        let command = &mut self.inner;
        let output: Output;

        if self.inspector.should_pass_stdin {
            let mut child = command
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .map_err(|error| ProcessError::Capture {
                    bin: self.get_bin_name(),
                    error,
                })?;

            self.write_input_to_child(&mut child).await?;

            output = child
                .wait_with_output()
                .await
                .map_err(|error| ProcessError::Capture {
                    bin: self.get_bin_name(),
                    error,
                })?;
        } else {
            output = command
                .output()
                .await
                .map_err(|error| ProcessError::Capture {
                    bin: self.get_bin_name(),
                    error,
                })?;
        }

        self.handle_nonzero_status(&output)?;

        Ok(output)
    }

    fn get_bin_name(&self) -> String {
        self.inner
            .as_std()
            .get_program()
            .to_string_lossy()
            .to_string()
    }

    fn handle_nonzero_status(&self, output: &Output) -> Result<(), ProcessError> {
        if self.inspector.should_error_nonzero && !output.status.success() {
            return Err(output_to_error(self.get_bin_name(), output, true));
        }

        Ok(())
    }

    async fn write_input_to_child(&self, child: &mut Child) -> Result<(), ProcessError> {
        let input = self.inspector.get_input_line().unwrap_or_default();

        let mut stdin = child.stdin.take().unwrap_or_else(|| {
            panic!("Unable to write stdin: {input}");
        });

        stdin
            .write_all(input.as_bytes())
            .await
            .map_err(|error| ProcessError::WriteInput {
                bin: self.get_bin_name(),
                error,
            })?;

        drop(stdin);

        Ok(())
    }
}
