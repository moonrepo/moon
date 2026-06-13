use crate::command::{Command, CommandExecutable, Env};
use crate::helpers::format_command_line;
use crate::output::Output;
use crate::process_error::ProcessError;
use crate::process_registry::ProcessRegistry;
use crate::shared_child::SharedChild;
use miette::IntoDiagnostic;
use moon_common::color;
use moon_console::ConsoleStream;
use moon_env_var::GlobalEnvBag;
use rustc_hash::FxHashMap;
use starbase_shell::join_exe_args;
use std::env;
use std::ffi::{OsStr, OsString};
use std::io;
use std::path::PathBuf;
use std::process::{Command as StdCommand, Stdio};
use std::time::Instant;
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command as TokioCommand};
use tokio::task::{self, JoinHandle};
use tracing::{debug, enabled, trace};

impl Command {
    pub async fn exec_capture_output(&mut self) -> miette::Result<Output> {
        if self.continuous_pipe {
            return self.exec_capture_continuous_output().await;
        }

        let registry = ProcessRegistry::instance();
        let instant = Instant::now();
        let mut command = self.create_async_command()?;

        let child = if self.should_pass_stdin() {
            command
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());

            let mut child = command.spawn().map_err(|error| ProcessError::Capture {
                bin: self.get_bin_name(),
                error: Box::new(error),
            })?;

            self.write_input_to_child(&mut child).await?;

            child
        } else {
            command.stdout(Stdio::piped()).stderr(Stdio::piped());

            command.spawn().map_err(|error| ProcessError::Capture {
                bin: self.get_bin_name(),
                error: Box::new(error),
            })?
        };

        let shared_child = registry.add_running(child).await;

        self.pre_log_command(&shared_child);

        let result = shared_child
            .wait_with_output()
            .await
            .map_err(|error| ProcessError::Capture {
                bin: self.get_bin_name(),
                error: Box::new(error),
            });

        self.post_log_command(&shared_child, instant);

        registry.remove_running(shared_child).await;

        let output = result?;

        self.handle_nonzero_status(&output, true)?;

        Ok(output)
    }

    pub async fn exec_capture_continuous_output(&mut self) -> miette::Result<Output> {
        let registry = ProcessRegistry::instance();
        let instant = Instant::now();
        let mut command = self.create_async_command()?;

        command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let child = command.spawn().map_err(|error| ProcessError::Capture {
            bin: self.get_bin_name(),
            error: Box::new(error),
        })?;

        let shared_child = registry.add_running(child).await;
        let stdin = shared_child.take_stdin().await;
        let stdout = shared_child.take_stdout().await;
        let stderr = shared_child.take_stderr().await;

        self.pre_log_command(&shared_child);

        let items = self.input.drain(..).collect::<Vec<_>>();
        let bin_name = self.get_bin_name();

        let stdin_handle: JoinHandle<miette::Result<()>> = task::spawn(async move {
            if let Some(mut stdin) = stdin {
                for item in items {
                    stdin
                        .write_all(item.as_encoded_bytes())
                        .await
                        .map_err(|error| ProcessError::WriteInput {
                            bin: bin_name.clone(),
                            error: Box::new(error),
                        })?;
                }

                drop(stdin);
            }

            Ok(())
        });

        let stdout_handle = spawn_capture_lines(stdout, "stdout");
        let stderr_handle = spawn_capture_lines(stderr, "stderr");

        // Attempt to create the child output
        let result = shared_child
            .wait()
            .await
            .map_err(|error| ProcessError::Capture {
                bin: self.get_bin_name(),
                error: Box::new(error),
            });

        self.post_log_command(&shared_child, instant);

        registry.remove_running(shared_child).await;

        let exit = result?;

        stdin_handle.await.into_diagnostic()??;

        let output = Output {
            exit,
            stdout: stdout_handle
                .await
                .into_diagnostic()?
                .join("\n")
                .into_bytes(),
            stderr: stderr_handle
                .await
                .into_diagnostic()?
                .join("\n")
                .into_bytes(),
        };

        self.handle_nonzero_status(&output, true)?;

        Ok(output)
    }

    pub async fn exec_stream_output(&mut self) -> miette::Result<Output> {
        let registry = ProcessRegistry::instance();
        let instant = Instant::now();
        let mut command = self.create_async_command()?;

        let child = if self.should_pass_stdin() {
            command.stdin(Stdio::piped());

            let mut child = command.spawn().map_err(|error| ProcessError::Stream {
                bin: self.get_bin_name(),
                error: Box::new(error),
            })?;

            self.write_input_to_child(&mut child).await?;

            child
        } else {
            command.spawn().map_err(|error| ProcessError::Stream {
                bin: self.get_bin_name(),
                error: Box::new(error),
            })?
        };

        let shared_child = registry.add_running(child).await;

        self.pre_log_command(&shared_child);

        let result = shared_child
            .wait()
            .await
            .map_err(|error| ProcessError::Stream {
                bin: self.get_bin_name(),
                error: Box::new(error),
            });

        self.post_log_command(&shared_child, instant);

        registry.remove_running(shared_child).await;

        let exit = result?;
        let output = Output {
            exit,
            stderr: vec![],
            stdout: vec![],
        };

        self.handle_nonzero_status(&output, false)?;

        Ok(output)
    }

    #[allow(unused, unreachable_code)]
    pub async fn exec_stream_and_capture_output(&mut self) -> miette::Result<Output> {
        return self.exec_stream_and_capture_output_bytes().await;

        let registry = ProcessRegistry::instance();
        let instant = Instant::now();
        let mut command = self.create_async_command()?;

        command
            .stdin(if self.should_pass_stdin() {
                Stdio::piped()
            } else {
                Stdio::inherit()
            })
            .stderr(Stdio::piped())
            .stdout(Stdio::piped());

        let mut child = command
            .spawn()
            .map_err(|error| ProcessError::StreamCapture {
                bin: self.get_bin_name(),
                error: Box::new(error),
            })?;

        if self.should_pass_stdin() {
            self.write_input_to_child(&mut child).await?;
        }

        let shared_child = registry.add_running(child).await;

        self.pre_log_command(&shared_child);

        // We need to log the child process output to the parent terminal
        // AND capture stdout/stderr so that we can cache it for future runs.
        // This isn't supported natively by `Stdio`, so we stream each pipe
        // through a task that writes to the console while capturing lines.
        // https://stackoverflow.com/a/49063262
        let console = self
            .console
            .as_ref()
            .expect("A console is required when streaming output!");
        let prefix = self.get_prefix().map(|prefix| prefix.to_owned());

        let stderr_handle = spawn_stream_capture_lines(
            shared_child.take_stderr().await,
            console.stderr(),
            prefix.clone(),
            "stderr",
        );
        let stdout_handle = spawn_stream_capture_lines(
            shared_child.take_stdout().await,
            console.stdout(),
            prefix,
            "stdout",
        );

        // Wait for the pipes to hit EOF before waiting on the child,
        // otherwise output may be lost
        let captured_stderr = stderr_handle.await.unwrap_or_default();
        let captured_stdout = stdout_handle.await.unwrap_or_default();

        // Attempt to create the child output
        let result = shared_child
            .wait()
            .await
            .map_err(|error| ProcessError::StreamCapture {
                bin: self.get_bin_name(),
                error: Box::new(error),
            });

        self.post_log_command(&shared_child, instant);

        registry.remove_running(shared_child).await;

        let exit = result?;
        let output = Output {
            exit,
            stdout: captured_stdout.join("\n").into_bytes(),
            stderr: captured_stderr.join("\n").into_bytes(),
        };

        self.handle_nonzero_status(&output, true)?;

        Ok(output)
    }

    /// A variant of [`Command::exec_stream_and_capture_output`] that tees raw
    /// bytes instead of lines. Partial lines and carriage return based redraws
    /// (progress bars, spinners) stream to the console in real time, non-UTF-8
    /// output is preserved, and the captured output is byte-exact, except that
    /// redraw frames are collapsed so cache replays only render the final frame.
    pub async fn exec_stream_and_capture_output_bytes(&mut self) -> miette::Result<Output> {
        let registry = ProcessRegistry::instance();
        let instant = Instant::now();
        let mut command = self.create_async_command()?;

        command
            .stdin(if self.should_pass_stdin() {
                Stdio::piped()
            } else {
                Stdio::inherit()
            })
            .stderr(Stdio::piped())
            .stdout(Stdio::piped());

        let mut child = command
            .spawn()
            .map_err(|error| ProcessError::StreamCapture {
                bin: self.get_bin_name(),
                error: Box::new(error),
            })?;

        if self.should_pass_stdin() {
            self.write_input_to_child(&mut child).await?;
        }

        let shared_child = registry.add_running(child).await;

        self.pre_log_command(&shared_child);

        let console = self
            .console
            .as_ref()
            .expect("A console is required when streaming output!");
        let prefix = self.get_prefix().map(|prefix| prefix.to_owned());

        let stderr_handle = spawn_stream_capture_bytes(
            shared_child.take_stderr().await,
            console.stderr(),
            prefix.clone(),
            "stderr",
        );
        let stdout_handle = spawn_stream_capture_bytes(
            shared_child.take_stdout().await,
            console.stdout(),
            prefix,
            "stdout",
        );

        // Wait for the pipes to hit EOF before waiting on the child,
        // otherwise output may be lost
        let captured_stderr = stderr_handle.await.unwrap_or_default();
        let captured_stdout = stdout_handle.await.unwrap_or_default();

        // Attempt to create the child output
        let result = shared_child
            .wait()
            .await
            .map_err(|error| ProcessError::StreamCapture {
                bin: self.get_bin_name(),
                error: Box::new(error),
            });

        self.post_log_command(&shared_child, instant);

        registry.remove_running(shared_child).await;

        let exit = result?;
        let output = Output {
            exit,
            stdout: captured_stdout,
            stderr: captured_stderr,
        };

        self.handle_nonzero_status(&output, true)?;

        Ok(output)
    }

    fn create_sync_command(&self) -> miette::Result<StdCommand> {
        // When the command is wrapped in a shell, we need to create a single
        // string of the full command line with args quoted correctly, as
        // it's passed as a single argument to the shell: `bash -c "command line"`
        let mut command = if self.shell.is_some() || self.exe.requires_shell() {
            let shell = self.shell.unwrap_or_default().build();

            let script = match &self.exe {
                CommandExecutable::Binary(bin) => join_exe_args(&shell, bin, &self.args, false),
                CommandExecutable::Script(script) => script.to_owned(),
            };

            shell.create_wrapped_command_with(script)
        }
        // When the command is not in a shell, we can create a standard command
        // and pass the non-quoted args separately
        else {
            let mut command = StdCommand::new(self.exe.as_os_str());

            for arg in &self.args {
                command.arg(&arg.value);
            }

            command
        };

        // Inherit added/removed vars first
        let bag = GlobalEnvBag::instance();

        bag.list_added(|key, value| {
            command.env(key, value);
        });

        bag.list_removed(|key| {
            command.env_remove(key);
        });

        // Then set explicit vars
        for (key, value) in &self.env {
            match value {
                Env::Set(value) => {
                    command.env(key, value);
                }
                Env::SetIfMissing(value) => {
                    if !bag.has(key) {
                        command.env(key, value);
                    }
                }
                Env::Unset => {
                    command.env_remove(key);
                }
            };
        }

        if let Some(cwd) = &self.cwd {
            command.current_dir(cwd);
            command.env("PWD", cwd);
        }

        // And lastly inherit lookup paths
        let path_key = OsString::from("PATH");

        if !self.env.contains_key(&path_key) && !self.paths.is_empty() {
            let mut paths = self.paths.iter().map(PathBuf::from).collect::<Vec<_>>();

            if let Some(path_value) = env::var_os(&path_key) {
                paths.extend(env::split_paths(&path_value));
            }

            command.env(path_key, env::join_paths(paths).into_diagnostic()?);
        }

        Ok(command)
    }

    fn create_async_command(&self) -> miette::Result<TokioCommand> {
        Ok(TokioCommand::from(self.create_sync_command()?))
    }

    fn handle_nonzero_status(&mut self, output: &Output, with_message: bool) -> miette::Result<()> {
        if self.should_error_nonzero() && !output.success() {
            return Err(output.to_error(self.get_bin_name(), with_message).into());
        }

        Ok(())
    }

    fn pre_log_command(&self, child: &SharedChild) {
        let bag = GlobalEnvBag::instance();
        let key = OsString::from("MOON_WORKSPACE_ROOT");

        // Determine workspace root and working dir
        let workspace_root = if let Some(root) = self.env.get(&key).and_then(|var| var.get_value())
        {
            PathBuf::from(root)
        } else if let Some(root) = bag.get(&key) {
            PathBuf::from(root)
        } else {
            env::current_dir().unwrap_or_default()
        };

        let working_dir = PathBuf::from(self.cwd.as_deref().unwrap_or(workspace_root.as_os_str()));

        // Print the command line to the console
        if let Some(console) = self.console.as_ref()
            && self.print_command
            && !console.out.is_quiet()
        {
            let command_line = self.get_command_line(false, false);

            let _ = console.out.write_line(format_command_line(
                &command_line,
                &workspace_root,
                &working_dir,
            ));
        }

        // Avoid all this overhead if we're not logging
        if !enabled!(tracing::Level::DEBUG) {
            return;
        }

        let debug_env = bag.should_debug_process_env();
        let env_vars: FxHashMap<&OsString, &OsString> = self
            .env
            .iter()
            .filter_map(|(key, value)| {
                if value == &Env::Unset {
                    None
                } else if debug_env || key.to_str().is_some_and(|k| k.starts_with("MOON_")) {
                    Some((key, value.get_value().unwrap()))
                } else {
                    None
                }
            })
            .collect();

        let shell = self.shell.as_ref().map(|sh| sh.to_string());
        let input_size = (!self.input.is_empty()).then(|| self.get_input_size());
        let command_line = self.get_command_line(true, true);

        debug!(
            pid = child.id(),
            shell,
            env = ?env_vars,
            cwd = ?working_dir,
            input_size,
            "Running command {}",
            color::shell(command_line)
        );
    }

    fn post_log_command(&self, child: &SharedChild, instant: Instant) {
        trace!(pid = child.id(), "Ran command in {:?}", instant.elapsed());
    }

    async fn write_input_to_child(&self, child: &mut Child) -> miette::Result<()> {
        let mut stdin = child.stdin.take().expect("Unable to write stdin!");

        stdin
            .write_all(self.input.join(OsStr::new(" ")).as_encoded_bytes())
            .await
            .map_err(|error| ProcessError::WriteInput {
                bin: self.get_bin_name(),
                error: Box::new(error),
            })?;

        drop(stdin);

        Ok(())
    }
}

fn spawn_capture_lines<R>(reader: Option<R>, label: &'static str) -> JoinHandle<Vec<String>>
where
    R: AsyncRead + Unpin + Send + 'static,
{
    task::spawn(async move {
        let mut logs = vec![];

        let Some(reader) = reader else {
            return logs;
        };

        let mut lines = BufReader::new(reader).lines();

        loop {
            match lines.next_line().await {
                Ok(Some(line)) => logs.push(line),
                Ok(None) => break,
                Err(error) => {
                    trace!("Failed to read {label} line: {error}");
                    break;
                }
            }
        }

        logs
    })
}

fn spawn_stream_capture_bytes<R>(
    reader: Option<R>,
    stream: ConsoleStream,
    prefix: Option<String>,
    label: &'static str,
) -> JoinHandle<Vec<u8>>
where
    R: AsyncRead + Unpin + Send + 'static,
{
    task::spawn(async move {
        let mut captured = vec![];

        let Some(mut reader) = reader else {
            return captured;
        };

        let mut buf = [0u8; 8192];
        let mut at_line_start = true;

        loop {
            match reader.read(&mut buf).await {
                // EOF
                Ok(0) => break,
                Ok(read) => {
                    let chunk = &buf[..read];

                    // Stream raw bytes to the console so that partial lines
                    // and carriage return based redraws render in real time
                    let _ = stream.write_raw(|out| {
                        match &prefix {
                            Some(prefix) => {
                                for segment in chunk.split_inclusive(|byte| *byte == b'\n') {
                                    if at_line_start {
                                        out.extend_from_slice(prefix.as_bytes());
                                    }

                                    out.extend_from_slice(segment);
                                    at_line_start = segment.ends_with(b"\n");
                                }
                            }
                            None => {
                                out.extend_from_slice(chunk);
                            }
                        };

                        Ok(())
                    });

                    captured.extend_from_slice(chunk);
                }
                Err(error) if error.kind() == io::ErrorKind::Interrupted => {
                    continue;
                }
                Err(error) => {
                    trace!("Failed to read {label} chunk: {error}");
                    break;
                }
            }
        }

        // Flush any remaining buffered output to ensure all streamed
        // content is visible before the next flow is printed
        let _ = stream.flush();

        collapse_redraw_frames(captured)
    })
}

// Within each line, keep only the content after the last carriage return,
// so that redraw frames (progress bars, spinners) don't replay when the
// captured output is rendered from cache. Trailing `\r\n` line endings
// are not treated as redraws.
fn collapse_redraw_frames(data: Vec<u8>) -> Vec<u8> {
    if !data.contains(&b'\r') {
        return data;
    }

    let mut result = Vec::with_capacity(data.len());

    for line in data.split_inclusive(|byte| *byte == b'\n') {
        let (content, ending): (&[u8], &[u8]) = match line {
            [content @ .., b'\r', b'\n'] => (content, b"\r\n"),
            [content @ .., b'\n'] => (content, b"\n"),
            _ => (line, b""),
        };

        let frame = match content.iter().rposition(|byte| *byte == b'\r') {
            Some(index) => &content[index + 1..],
            None => content,
        };

        result.extend_from_slice(frame);
        result.extend_from_slice(ending);
    }

    result
}

fn spawn_stream_capture_lines<R>(
    reader: Option<R>,
    stream: ConsoleStream,
    prefix: Option<String>,
    label: &'static str,
) -> JoinHandle<Vec<String>>
where
    R: AsyncRead + Unpin + Send + 'static,
{
    task::spawn(async move {
        let mut captured_lines = vec![];

        let Some(reader) = reader else {
            return captured_lines;
        };

        let mut lines = BufReader::new(reader).lines();

        loop {
            match lines.next_line().await {
                Ok(Some(line)) => {
                    let _ = if let Some(prefix) = &prefix {
                        stream.write_line_with_prefix(&line, prefix)
                    } else {
                        stream.write_line(&line)
                    };

                    captured_lines.push(line);
                }
                Ok(None) => break,
                Err(error) => {
                    // Don't break on read errors - dropping the BufReader here would close
                    // the read-end of the pipe while the child process is still writing,
                    // causing it to receive EPIPE and potentially exit with a non-zero code.
                    // Continue reading until we get a natural EOF (Ok(None)).
                    trace!("Failed to read {label} line: {error}");
                }
            }
        }

        // Flush any remaining buffered output to ensure all streamed
        // content is visible before the next flow is printed
        let _ = stream.flush();

        captured_lines
    })
}
