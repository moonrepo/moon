use crate::command_inspector::CommandInspector;
use crate::output_to_error;
use crate::process_error::ProcessError;
use moon_console::Console;
use std::process::{Output, Stdio};
use std::sync::{Arc, RwLock};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::task;

pub struct AsyncCommand<'cmd> {
    pub console: Option<Arc<Console>>,
    pub inner: Command,
    pub inspector: CommandInspector<'cmd>,

    pub current_id: Option<u32>,
}

impl<'cmd> AsyncCommand<'cmd> {
    pub async fn exec_capture_output(&mut self) -> miette::Result<Output> {
        self.inspector.log_command();

        let command = &mut self.inner;
        let output: Output;

        if self.inspector.should_pass_stdin() {
            let mut child = command
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .map_err(|error| ProcessError::Capture {
                    bin: self.get_bin_name(),
                    error: Box::new(error),
                })?;

            self.write_input_to_child(&mut child).await?;

            self.current_id = child.id();

            output = child
                .wait_with_output()
                .await
                .map_err(|error| ProcessError::Capture {
                    bin: self.get_bin_name(),
                    error: Box::new(error),
                })?;
        } else {
            output = command
                .output()
                .await
                .map_err(|error| ProcessError::Capture {
                    bin: self.get_bin_name(),
                    error: Box::new(error),
                })?;
        }

        self.handle_nonzero_status(&output, true)?;

        Ok(output)
    }

    pub async fn exec_stream_output(&mut self) -> miette::Result<Output> {
        self.inspector.log_command();

        let command = &mut self.inner;
        let mut child: Child;

        if self.inspector.should_pass_stdin() {
            child =
                command
                    .stdin(Stdio::piped())
                    .spawn()
                    .map_err(|error| ProcessError::Stream {
                        bin: self.get_bin_name(),
                        error: Box::new(error),
                    })?;

            self.write_input_to_child(&mut child).await?;
        } else {
            child = command.spawn().map_err(|error| ProcessError::Stream {
                bin: self.get_bin_name(),
                error: Box::new(error),
            })?;
        };

        self.current_id = child.id();

        let status = child.wait().await.map_err(|error| ProcessError::Stream {
            bin: self.get_bin_name(),
            error: Box::new(error),
        })?;

        let output = Output {
            status,
            stderr: vec![],
            stdout: vec![],
        };

        self.handle_nonzero_status(&output, false)?;

        Ok(output)
    }

    pub async fn exec_stream_and_capture_output(&mut self) -> miette::Result<Output> {
        self.inspector.log_command();

        let command = &mut self.inner;

        let mut child = command
            .stdin(if self.inspector.should_pass_stdin() {
                Stdio::piped()
            } else {
                Stdio::inherit()
            })
            .stderr(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .map_err(|error| ProcessError::StreamCapture {
                bin: self.get_bin_name(),
                error: Box::new(error),
            })?;

        self.current_id = child.id();

        if self.inspector.should_pass_stdin() {
            self.write_input_to_child(&mut child).await?;
        }

        // We need to log the child process output to the parent terminal
        // AND capture stdout/stderr so that we can cache it for future runs.
        // This doesn't seem to be supported natively by `Stdio`, so I have
        // this *real ugly* implementation to solve it. There's gotta be a
        // better way to do this?
        // https://stackoverflow.com/a/49063262
        let stderr = BufReader::new(child.stderr.take().unwrap());
        let stdout = BufReader::new(child.stdout.take().unwrap());
        let mut handles = vec![];

        let captured_stderr = Arc::new(RwLock::new(vec![]));
        let captured_stdout = Arc::new(RwLock::new(vec![]));
        let captured_stderr_clone = Arc::clone(&captured_stderr);
        let captured_stdout_clone = Arc::clone(&captured_stdout);

        let prefix = Arc::new(self.inspector.get_prefix());
        let stderr_prefix = Arc::clone(&prefix);
        let stdout_prefix = Arc::clone(&prefix);

        let console = self
            .console
            .as_ref()
            .expect("A console is required when streaming output!");
        let stderr_stream = Arc::new(console.stderr().to_owned());
        let stdout_stream = Arc::new(console.stdout().to_owned());

        handles.push(task::spawn(async move {
            let mut lines = stderr.lines();
            let mut captured_lines = vec![];

            while let Ok(Some(line)) = lines.next_line().await {
                let _ = if stderr_prefix.is_empty() {
                    stderr_stream.write_line(&line)
                } else {
                    stderr_stream.write_line(format!("{stderr_prefix}{line}"))
                };

                captured_lines.push(line);
            }

            captured_stderr_clone
                .write()
                .unwrap()
                .extend(captured_lines);
        }));

        handles.push(task::spawn(async move {
            let mut lines = stdout.lines();
            let mut captured_lines = vec![];

            while let Ok(Some(line)) = lines.next_line().await {
                let _ = if stdout_prefix.is_empty() {
                    stdout_stream.write_line(&line)
                } else {
                    stdout_stream.write_line(format!("{stdout_prefix}{line}"))
                };

                captured_lines.push(line);
            }

            captured_stdout_clone
                .write()
                .unwrap()
                .extend(captured_lines);
        }));

        for handle in handles {
            let _ = handle.await;
        }

        // Attempt to create the child output
        let status = child
            .wait()
            .await
            .map_err(|error| ProcessError::StreamCapture {
                bin: self.get_bin_name(),
                error: Box::new(error),
            })?;

        let output = Output {
            status,
            stdout: captured_stdout.read().unwrap().join("\n").into_bytes(),
            stderr: captured_stderr.read().unwrap().join("\n").into_bytes(),
        };

        self.handle_nonzero_status(&output, true)?;

        Ok(output)
    }

    fn get_bin_name(&self) -> String {
        self.inner
            .as_std()
            .get_program()
            .to_string_lossy()
            .to_string()
    }

    fn handle_nonzero_status(&mut self, output: &Output, with_message: bool) -> miette::Result<()> {
        let bin_name = self.get_bin_name();

        #[cfg(unix)]
        {
            use std::os::unix::process::ExitStatusExt;
            use tracing::warn;

            if output.status.signal().is_some() || output.status.stopped_signal().is_some() {
                warn!(
                    command = self
                        .inspector
                        .get_command_line()
                        .main_command
                        .to_str()
                        .unwrap_or(&bin_name),
                    pid = self.current_id,
                    "Process unexpectedly exited: {}",
                    output.status
                );
            }
        }

        self.current_id = None;

        if self.inspector.should_error_nonzero() && !output.status.success() {
            return Err(output_to_error(bin_name, output, with_message).into());
        }

        Ok(())
    }

    async fn write_input_to_child(&self, child: &mut Child) -> miette::Result<()> {
        let input = &self.inspector.get_command_line().input;

        let mut stdin = child.stdin.take().unwrap_or_else(|| {
            panic!("Unable to write stdin: {}", input.to_string_lossy());
        });

        stdin
            .write_all(input.as_encoded_bytes())
            .await
            .map_err(|error| ProcessError::WriteInput {
                bin: self.get_bin_name(),
                error: Box::new(error),
            })?;

        drop(stdin);

        Ok(())
    }
}
