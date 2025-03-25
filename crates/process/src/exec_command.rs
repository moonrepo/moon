use crate::command::Command;
use crate::command_line::CommandLine;
// use crate::output_stream::capture_stream;
use crate::output_to_error;
use crate::process_error::ProcessError;
use crate::process_registry::ProcessRegistry;
use crate::shared_child::SharedChild;
use miette::IntoDiagnostic;
use moon_common::color;
use moon_env_var::GlobalEnvBag;
use rustc_hash::FxHashMap;
use std::env;
use std::ffi::{OsStr, OsString};
use std::path::PathBuf;
use std::process::{Output, Stdio};
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
        let (mut command, line, instant) = self.create_async_command();

        let child = if self.should_pass_stdin() {
            command
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());

            let mut child = command.spawn().map_err(|error| ProcessError::Capture {
                bin: self.get_bin_name(),
                error: Box::new(error),
            })?;

            self.write_input_to_child(&mut child, &line).await?;

            child
        } else {
            command.stdout(Stdio::piped()).stderr(Stdio::piped());

            command.spawn().map_err(|error| ProcessError::Capture {
                bin: self.get_bin_name(),
                error: Box::new(error),
            })?
        };

        let shared_child = registry.add_running(child).await;

        self.pre_log_command(&line, &shared_child);

        let result = shared_child
            .wait_with_output()
            .await
            .map_err(|error| ProcessError::Capture {
                bin: self.get_bin_name(),
                error: Box::new(error),
            });

        self.post_log_command(instant, &shared_child);

        registry.remove_running(shared_child).await;

        let output = result?;

        self.handle_nonzero_status(&output, true)?;

        Ok(output)
    }

    pub async fn exec_capture_continuous_output(&mut self) -> miette::Result<Output> {
        let registry = ProcessRegistry::instance();
        let (mut command, line, instant) = self.create_async_command();

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

        self.pre_log_command(&line, &shared_child);

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

            while let Ok(Some(line)) = lines.next_line().await {
                logs.push(line);
            }

            Ok(logs)
        });

        let stderr_handle: JoinHandle<miette::Result<Vec<String>>> = task::spawn(async move {
            let mut logs = vec![];
            let mut lines = BufReader::new(stderr.unwrap()).lines();

            while let Ok(Some(line)) = lines.next_line().await {
                logs.push(line);
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

        self.post_log_command(instant, &shared_child);

        registry.remove_running(shared_child).await;

        let status = result?;

        stdin_handle.await.into_diagnostic()??;

        let output = Output {
            status,
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
        let (mut command, line, instant) = self.create_async_command();

        let child = if self.should_pass_stdin() {
            command.stdin(Stdio::piped());

            let mut child = command.spawn().map_err(|error| ProcessError::Stream {
                bin: self.get_bin_name(),
                error: Box::new(error),
            })?;

            self.write_input_to_child(&mut child, &line).await?;

            child
        } else {
            command.spawn().map_err(|error| ProcessError::Stream {
                bin: self.get_bin_name(),
                error: Box::new(error),
            })?
        };

        let shared_child = registry.add_running(child).await;

        self.pre_log_command(&line, &shared_child);

        let result = shared_child
            .wait()
            .await
            .map_err(|error| ProcessError::Stream {
                bin: self.get_bin_name(),
                error: Box::new(error),
            });

        self.post_log_command(instant, &shared_child);

        registry.remove_running(shared_child).await;

        let status = result?;
        let output = Output {
            status,
            stderr: vec![],
            stdout: vec![],
        };

        self.handle_nonzero_status(&output, false)?;

        Ok(output)
    }

    pub async fn exec_stream_and_capture_output(&mut self) -> miette::Result<Output> {
        let registry = ProcessRegistry::instance();
        let (mut command, line, instant) = self.create_async_command();

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
            self.write_input_to_child(&mut child, &line).await?;
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

        self.pre_log_command(&line, &shared_child);

        // Attempt to create the child output
        let result = shared_child
            .wait()
            .await
            .map_err(|error| ProcessError::StreamCapture {
                bin: self.get_bin_name(),
                error: Box::new(error),
            });

        self.post_log_command(instant, &shared_child);

        registry.remove_running(shared_child).await;

        let status = result?;
        let output = Output {
            status,
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

    fn create_async_command(&self) -> (TokioCommand, CommandLine, Instant) {
        let command_line = self.create_command_line();

        let mut command = TokioCommand::new(&command_line.command[0]);
        command.args(&command_line.command[1..]);

        // Inherit global env first
        let bag = GlobalEnvBag::instance();

        bag.list_added(|key, value| {
            command.env(key, value);
        });

        bag.list_removed(|key| {
            command.env_remove(key);
        });

        // Then inherit local so we can override global
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

        // #[cfg(windows)]
        // {
        //     command.creation_flags(windows_sys::Win32::System::Threading::CREATE_NEW_PROCESS_GROUP);
        // }

        (command, command_line, Instant::now())
    }

    fn create_command_line(&self) -> CommandLine {
        CommandLine::new(self)
    }

    fn handle_nonzero_status(&mut self, output: &Output, with_message: bool) -> miette::Result<()> {
        if self.should_error_nonzero() && !output.status.success() {
            return Err(output_to_error(self.get_bin_name(), output, with_message).into());
        }

        Ok(())
    }

    fn pre_log_command(&self, line: &CommandLine, child: &SharedChild) {
        let bag = GlobalEnvBag::instance();

        let workspace_env_key = OsString::from("MOON_WORKSPACE_ROOT");
        let workspace_root = if let Some(Some(value)) = self.env.get(&workspace_env_key) {
            PathBuf::from(value)
        } else {
            bag.get(&workspace_env_key).map_or_else(
                || env::current_dir().unwrap_or(PathBuf::from(".")),
                PathBuf::from,
            )
        };
        let working_dir = PathBuf::from(self.cwd.as_deref().unwrap_or(workspace_root.as_os_str()));

        if let Some(console) = self.console.as_ref() {
            if self.print_command && !console.out.is_quiet() {
                let _ = console.out.write_line(CommandLine::format(
                    &line.get_line(false, false),
                    &workspace_root,
                    &working_dir,
                ));
            }
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
                if value.is_none() {
                    None
                } else if debug_env
                    || key
                        .to_str()
                        .map(|k| k.starts_with("MOON_"))
                        .unwrap_or_default()
                {
                    Some((key, value.as_ref().unwrap()))
                } else {
                    None
                }
            })
            .collect();

        let debug_input = bag.should_debug_process_input();
        let input_size: Option<usize> = if self.input.is_empty() {
            None
        } else {
            Some(self.input.iter().map(|i| i.len()).sum())
        };

        let mut line = line.to_string();
        let line_size = line.len();

        if line_size > 1000 && !debug_input {
            line.truncate(1000);
            line.push_str(&format!(" ... (and {} more bytes)", line_size - 1000));
        }

        debug!(
            pid = child.id(),
            shell = self.shell.as_ref().map(|sh| &sh.bin_name),
            env = ?env_vars,
            cwd = ?working_dir,
            input_size,
            "Running command {}",
            color::shell(line)
        );
    }

    fn post_log_command(&self, instant: Instant, child: &SharedChild) {
        trace!(pid = child.id(), "Ran command in {:?}", instant.elapsed());
    }

    async fn write_input_to_child(
        &self,
        child: &mut Child,
        line: &CommandLine,
    ) -> miette::Result<()> {
        let mut stdin = child.stdin.take().expect("Unable to write stdin!");

        stdin
            .write_all(line.input.join(OsStr::new(" ")).as_encoded_bytes())
            .await
            .map_err(|error| ProcessError::WriteInput {
                bin: self.get_bin_name(),
                error: Box::new(error),
            })?;

        drop(stdin);

        Ok(())
    }
}
