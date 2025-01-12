use crate::command::Command;
use crate::command_line::CommandLine;
use crate::output_to_error;
use crate::process_error::ProcessError;
use moon_common::color;
use rustc_hash::FxHashMap;
use std::env;
use std::ffi::{OsStr, OsString};
use std::path::PathBuf;
use std::process::{Output, Stdio};
use std::sync::{Arc, RwLock};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command as AsyncCommand};
use tokio::task;
use tracing::{debug, enabled};

impl Command {
    pub async fn exec_capture_output(&mut self) -> miette::Result<Output> {
        let (mut command, line) = self.create_async_command();
        let output: Output;

        if self.should_pass_stdin() {
            let mut child = command
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .map_err(|error| ProcessError::Capture {
                    bin: self.get_bin_name(),
                    error: Box::new(error),
                })?;

            self.write_input_to_child(&mut child, &line).await?;

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
        let (mut command, line) = self.create_async_command();
        let mut child: Child;

        if self.should_pass_stdin() {
            child =
                command
                    .stdin(Stdio::piped())
                    .spawn()
                    .map_err(|error| ProcessError::Stream {
                        bin: self.get_bin_name(),
                        error: Box::new(error),
                    })?;

            self.write_input_to_child(&mut child, &line).await?;
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
        let (mut command, line) = self.create_async_command();

        let mut child = command
            .stdin(if self.should_pass_stdin() {
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

        if self.should_pass_stdin() {
            self.write_input_to_child(&mut child, &line).await?;
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

        let prefix = Arc::new(self.get_prefix().map(|prefix| prefix.to_owned()));
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
                let _ = if let Some(prefix) = &*stderr_prefix {
                    stderr_stream.write_line_with_prefix(&line, prefix)
                } else {
                    stderr_stream.write_line(&line)
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
                let _ = if let Some(prefix) = &*stdout_prefix {
                    stdout_stream.write_line_with_prefix(&line, prefix)
                } else {
                    stdout_stream.write_line(&line)
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

    fn create_async_command(&self) -> (AsyncCommand, CommandLine) {
        let command_line = self.create_command_line();

        self.log_command(&command_line);

        let mut command = AsyncCommand::new(&command_line.command[0]);
        command.args(&command_line.command[1..]);
        command.kill_on_drop(true);

        for (key, value) in &self.env {
            if let Some(value) = value {
                command.env(key, value);
            } else {
                command.env_remove(key);
            }
        }

        if let Some(cwd) = &self.cwd {
            command.current_dir(cwd);
        }

        (command, command_line)
    }

    fn create_command_line(&self) -> CommandLine {
        CommandLine::new(self)
    }

    fn handle_nonzero_status(&mut self, output: &Output, with_message: bool) -> miette::Result<()> {
        self.current_id = None;

        if self.should_error_nonzero() && !output.status.success() {
            return Err(output_to_error(self.get_bin_name(), output, with_message).into());
        }

        Ok(())
    }

    fn log_command(&self, line: &CommandLine) {
        let workspace_env_key = OsString::from("MOON_WORKSPACE_ROOT");
        let workspace_root = if let Some(Some(value)) = self.env.get(&workspace_env_key) {
            PathBuf::from(value)
        } else {
            env::var_os(&workspace_env_key).map_or_else(
                || env::current_dir().unwrap_or(PathBuf::from(".")),
                PathBuf::from,
            )
        };
        let working_dir = PathBuf::from(self.cwd.as_deref().unwrap_or(workspace_root.as_os_str()));

        if self.print_command {
            // if let Some(cmd_line) = self.get_command_line().main_command.to_str() {
            if let Some(console) = self.console.as_ref() {
                if !console.out.is_quiet() {
                    let _ = console.out.write_line(CommandLine::format(
                        &line.to_string(),
                        &workspace_root,
                        &working_dir,
                    ));
                }
            }
            // }
        }

        // Avoid all this overhead if we're not logging
        if !enabled!(tracing::Level::DEBUG) {
            return;
        }

        let debug_env = env::var("MOON_DEBUG_PROCESS_ENV").is_ok();
        let env_vars = self
            .env
            .iter()
            .filter(|(key, _)| {
                if debug_env {
                    true
                } else {
                    key.to_str()
                        .map(|k| k.starts_with("MOON_"))
                        .unwrap_or_default()
                }
            })
            .collect::<FxHashMap<_, _>>();

        debug!(
            env_vars = ?env_vars,
            working_dir = ?working_dir,
            "Running command {}",
            color::shell(line.to_string())
        );
    }

    async fn write_input_to_child(
        &self,
        child: &mut Child,
        line: &CommandLine,
    ) -> miette::Result<()> {
        let input = line.input.join(OsStr::new(" "));

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
