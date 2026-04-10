use crate::command::{Command, CommandExecutable, Env};
use crate::helpers::format_command_line;
// use crate::output_stream::capture_stream;
use crate::output::Output;
use crate::process_error::ProcessError;
use crate::process_registry::ProcessRegistry;
use crate::shared_child::SharedChild;
use miette::IntoDiagnostic;
use moon_common::color;
use moon_env_var::GlobalEnvBag;
use rustc_hash::FxHashMap;
use starbase_shell::join_exe_args;
use std::env;
use std::ffi::{OsStr, OsString};
use std::path::PathBuf;
use std::process::{Command as StdCommand, Stdio};
use std::sync::{Arc, RwLock};
use std::time::Instant;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
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

        let stdout_handle: JoinHandle<miette::Result<Vec<String>>> = task::spawn(async move {
            let mut logs = vec![];
            let mut lines = BufReader::new(stdout.unwrap()).lines();

            loop {
                match lines.next_line().await {
                    Ok(Some(line)) => logs.push(line),
                    Ok(None) => break,
                    Err(error) => {
                        trace!("Failed to read stdout line: {error}");
                        break;
                    }
                }
            }

            Ok(logs)
        });

        let stderr_handle: JoinHandle<miette::Result<Vec<String>>> = task::spawn(async move {
            let mut logs = vec![];
            let mut lines = BufReader::new(stderr.unwrap()).lines();

            loop {
                match lines.next_line().await {
                    Ok(Some(line)) => logs.push(line),
                    Ok(None) => break,
                    Err(error) => {
                        trace!("Failed to read stderr line: {error}");
                        break;
                    }
                }
            }

            Ok(logs)
        });

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
                .into_diagnostic()??
                .join("\n")
                .into_bytes(),
            stderr: stderr_handle
                .await
                .into_diagnostic()??
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

    pub async fn exec_stream_and_capture_output(&mut self) -> miette::Result<Output> {
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

        // We need to log the child process output to the parent terminal
        // AND capture stdout/stderr so that we can cache it for future runs.
        // This doesn't seem to be supported natively by `Stdio`, so I have
        // this *real ugly* implementation to solve it. There's gotta be a
        // better way to do this?
        // https://stackoverflow.com/a/49063262
        let stderr = BufReader::new(shared_child.take_stderr().await.unwrap());
        let stdout = BufReader::new(shared_child.take_stdout().await.unwrap());
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

            loop {
                match lines.next_line().await {
                    Ok(Some(line)) => {
                        let _ = if let Some(prefix) = &*stderr_prefix {
                            stderr_stream.write_line_with_prefix(&line, prefix)
                        } else {
                            stderr_stream.write_line(&line)
                        };

                        captured_lines.push(line);
                    }
                    Ok(None) => break,
                    Err(error) => {
                        // Don't break on read errors - dropping the BufReader here would close
                        // the read-end of the pipe while the child process is still writing,
                        // causing it to receive EPIPE and potentially exit with a non-zero code.
                        // Continue reading until we get a natural EOF (Ok(None)).
                        trace!("Failed to read stderr line: {error}");
                    }
                }
            }

            // Flush any remaining buffered output to ensure all streamed
            // content is visible before the next flow is printed
            let _ = stderr_stream.flush();

            captured_stderr_clone
                .write()
                .unwrap()
                .extend(captured_lines);
        }));

        handles.push(task::spawn(async move {
            let mut lines = stdout.lines();
            let mut captured_lines = vec![];

            loop {
                match lines.next_line().await {
                    Ok(Some(line)) => {
                        let _ = if let Some(prefix) = &*stdout_prefix {
                            stdout_stream.write_line_with_prefix(&line, prefix)
                        } else {
                            stdout_stream.write_line(&line)
                        };

                        captured_lines.push(line);
                    }
                    Ok(None) => break,
                    Err(error) => {
                        // Don't break on read errors - dropping the BufReader here would close
                        // the read-end of the pipe while the child process is still writing,
                        // causing it to receive EPIPE and potentially exit with a non-zero code.
                        // Continue reading until we get a natural EOF (Ok(None)).
                        trace!("Failed to read stdout line: {error}");
                    }
                }
            }

            // Flush any remaining buffered output to ensure all streamed
            // content is visible before the next flow is printed
            let _ = stdout_stream.flush();

            captured_stdout_clone
                .write()
                .unwrap()
                .extend(captured_lines);
        }));

        for handle in handles {
            let _ = handle.await;
        }

        self.pre_log_command(&shared_child);

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
            stdout: captured_stdout.read().unwrap().join("\n").into_bytes(),
            stderr: captured_stderr.read().unwrap().join("\n").into_bytes(),
        };

        self.handle_nonzero_status(&output, true)?;

        Ok(output)
    }

    // pub async fn exec_stream_and_capture_output_new(&mut self) -> miette::Result<Output> {
    //     let registry = ProcessRegistry::instance();
    //     let (mut command, line) = self.create_async_command();

    //     let mut child = command
    //         .stdin(if self.should_pass_stdin() {
    //             Stdio::piped()
    //         } else {
    //             Stdio::inherit()
    //         })
    //         .stderr(Stdio::piped())
    //         .stdout(Stdio::piped())
    //         .spawn()
    //         .map_err(|error| ProcessError::StreamCapture {
    //             bin: self.get_bin_name(),
    //             error: Box::new(error),
    //         })?;

    //     if self.should_pass_stdin() {
    //         self.write_input_to_child(&mut child, &line).await?;
    //     }

    //     let shared_child = registry.add_running(child).await;

    //     // Stream and attempt to capture the output
    //     let stderr = shared_child.take_stderr().await.unwrap();
    //     let mut stderr_buffer = Vec::new();
    //     let mut stderr_pos = 0;

    //     let stdout = shared_child.take_stdout().await.unwrap();
    //     let mut stdout_buffer = Vec::new();
    //     let mut stdout_pos = 0;

    //     let prefix = self.get_prefix();
    //     let console = self
    //         .console
    //         .as_ref()
    //         .expect("A console is required when streaming output!");

    //     capture_stream(stdout, stderr, &mut |is_out, data, eof| {
    //         let (pos, buf) = if is_out {
    //             (&mut stdout_pos, &mut stdout_buffer)
    //         } else {
    //             (&mut stderr_pos, &mut stderr_buffer)
    //         };

    //         let idx = if eof {
    //             data.len()
    //         } else {
    //             match data[*pos..].iter().rposition(|b| *b == b'\n') {
    //                 Some(i) => *pos + i + 1,
    //                 None => {
    //                     *pos = data.len();
    //                     return;
    //                 }
    //             }
    //         };

    //         let new_lines = &data[..idx];

    //         for line in String::from_utf8_lossy(new_lines).lines() {
    //             let stream = if is_out { &console.out } else { &console.err };

    //             let _ = if let Some(p) = &prefix {
    //                 stream.write_line_with_prefix(line.trim(), p)
    //             } else {
    //                 stream.write_line(line.trim())
    //             };
    //         }

    //         buf.extend(new_lines);
    //         data.drain(..idx);
    //         *pos = 0;
    //     })
    //     .await
    //     .map_err(|error| ProcessError::StreamCapture {
    //         bin: self.get_bin_name(),
    //         error: Box::new(error),
    //     })?;

    // self.log_command(&line, &shared_child);

    //     // Attempt to create the child output
    //     let result = shared_child
    //         .wait()
    //         .await
    //         .map_err(|error| ProcessError::StreamCapture {
    //             bin: self.get_bin_name(),
    //             error: Box::new(error),
    //         });

    //     registry.remove_running(shared_child).await;

    //     let status = result?;
    //     let output = Output {
    //         status,
    //         stdout: stdout_buffer,
    //         stderr: stderr_buffer,
    //     };

    //     self.handle_nonzero_status(&output, true)?;

    //     Ok(output)
    // }

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
