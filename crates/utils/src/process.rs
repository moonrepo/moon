use crate::{is_ci, is_test_env, path, shell};
use moon_error::{map_io_to_process_error, MoonError};
use moon_logger::{color, logging_enabled, pad_str, trace, Alignment};
use std::ffi::OsStr;
use std::path::Path;
use std::sync::{Arc, RwLock};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command as TokioCommand};
use tokio::task;

pub use shell_words::{join as join_args, split as split_args, ParseError as ArgsParseError};
pub use std::process::{ExitStatus, Output, Stdio};

pub fn is_windows_script(bin: &str) -> bool {
    bin.ends_with(".cmd") || bin.ends_with(".bat") || bin.ends_with(".ps1")
}

pub fn output_to_string(data: &[u8]) -> String {
    String::from_utf8(data.to_vec()).unwrap_or_default()
}

pub fn output_to_trimmed_string(data: &[u8]) -> String {
    output_to_string(data).trim().to_owned()
}

#[derive(Debug)]
pub struct Command {
    bin: String,

    cmd: TokioCommand,

    /// Convert non-zero exits to errors.
    error_on_nonzero: bool,

    /// Values to pass to stdin.
    input: Vec<u8>,

    /// Arguments will be passed via stdin to the command.
    pass_args_stdin: bool,

    /// Prefix to prepend to all log lines.
    prefix: Option<String>,
}

// This is rather annoying that we have to re-implement all these methods,
// but the encapsulation this struct provides is necessary.
impl Command {
    pub fn new<S: AsRef<OsStr>>(bin: S) -> Self {
        let bin = bin.as_ref();

        let mut command = Command {
            bin: String::from(bin.to_string_lossy()),
            cmd: TokioCommand::new(&bin),
            error_on_nonzero: true,
            input: vec![],
            pass_args_stdin: false,
            prefix: None,
        };

        // Referencing a batch script needs to be ran with a shell
        if is_windows_script(&command.bin) {
            let (bin_name, cmd) = shell::create_windows_shell();

            command.bin = bin_name;
            command.cmd = cmd;
            command.pass_args_stdin = true;
            command.arg(bin);
        }

        command
    }

    pub fn arg<S: AsRef<OsStr>>(&mut self, arg: S) -> &mut Command {
        if self.pass_args_stdin {
            self.input
                .extend(arg.as_ref().to_str().unwrap_or_default().as_bytes());
            self.input.extend(b" "); // Space between args
        } else {
            self.cmd.arg(arg);
        }

        self
    }

    pub fn args<I, S>(&mut self, args: I) -> &mut Command
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        if self.pass_args_stdin {
            for arg in args {
                self.arg(arg);
            }
        } else {
            self.cmd.args(args);
        }

        self
    }

    pub fn cwd<P: AsRef<Path>>(&mut self, dir: P) -> &mut Command {
        self.cmd.current_dir(dir);
        self
    }

    pub fn env<K, V>(&mut self, key: K, val: V) -> &mut Command
    where
        K: AsRef<OsStr>,
        V: AsRef<OsStr>,
    {
        self.cmd.env(key, val);
        self
    }

    pub fn envs<I, K, V>(&mut self, vars: I) -> &mut Command
    where
        I: IntoIterator<Item = (K, V)>,
        K: AsRef<OsStr>,
        V: AsRef<OsStr>,
    {
        self.cmd.envs(vars);
        self
    }

    pub fn input(&mut self, input: &[u8]) -> &mut Command {
        self.input.extend(input);
        self
    }

    pub async fn exec_capture_output(&mut self) -> Result<Output, MoonError> {
        self.log_command_info();

        let error_handler = |e| map_io_to_process_error(e, &self.bin);
        let output: Output;

        if self.input.is_empty() {
            output = self.cmd.output().await.map_err(error_handler)?;
        } else {
            let mut child = self
                .cmd
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .map_err(error_handler)?;

            self.write_input_to_child(&mut child).await?;

            output = child.wait_with_output().await.map_err(error_handler)?;
        }

        self.handle_nonzero_status(&output)?;

        Ok(output)
    }

    pub async fn exec_stream_output(&mut self) -> Result<ExitStatus, MoonError> {
        self.log_command_info();

        let error_handler = |e| map_io_to_process_error(e, &self.bin);
        let mut child: Child;

        if self.input.is_empty() {
            child = self.cmd.spawn().map_err(error_handler)?;
        } else {
            child = self
                .cmd
                .stdin(Stdio::piped())
                .spawn()
                .map_err(error_handler)?;

            self.write_input_to_child(&mut child).await?;
        };

        let status = child.wait().await.map_err(error_handler)?;

        if self.error_on_nonzero && !status.success() {
            return Err(MoonError::ProcessNonZero(
                self.bin.clone(),
                status.code().unwrap_or(-1),
            ));
        }

        Ok(status)
    }

    #[track_caller]
    pub async fn exec_stream_and_capture_output(&mut self) -> Result<Output, MoonError> {
        self.log_command_info();

        let error_handler = |e| map_io_to_process_error(e, &self.bin);

        let mut child = self
            .cmd
            .stdin(Stdio::piped())
            .stderr(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .map_err(error_handler)?;

        if !self.input.is_empty() {
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

        let prefix: Arc<str> = self.prefix.clone().unwrap_or_default().into();
        let stderr_prefix = Arc::clone(&prefix);
        let stdout_prefix = Arc::clone(&prefix);

        handles.push(task::spawn(async move {
            let mut lines = stderr.lines();
            let mut captured_lines = vec![];

            while let Some(line) = lines.next_line().await.unwrap_or_default() {
                if stderr_prefix.is_empty() {
                    eprintln!("{}", line);
                } else {
                    eprintln!("{} {}", stderr_prefix, line);
                }

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

            while let Some(line) = lines.next_line().await.unwrap_or_default() {
                if stdout_prefix.is_empty() {
                    println!("{}", line);
                } else {
                    println!("{} {}", stdout_prefix, line);
                }

                captured_lines.push(line);
            }

            captured_stdout_clone
                .write()
                .unwrap()
                .extend(captured_lines);
        }));

        for handle in handles {
            handle.await.expect("Failed to capture stdout/stderr");
        }

        // Attempt to capture the child output
        let mut output = child.wait_with_output().await.map_err(error_handler)?;

        if output.stderr.is_empty() {
            output.stderr = captured_stderr.read().unwrap().join("\n").into_bytes();
        }

        if output.stdout.is_empty() {
            output.stdout = captured_stdout.read().unwrap().join("\n").into_bytes();
        }

        self.handle_nonzero_status(&output)?;

        Ok(output)
    }

    pub fn get_command_line(&self) -> (String, Option<&Path>) {
        let cmd = &self.cmd.as_std();

        let args = cmd
            .get_args()
            .into_iter()
            .map(|a| a.to_str().unwrap_or("<unknown>"))
            .collect::<Vec<_>>();

        let line = if args.is_empty() {
            self.bin.to_owned()
        } else {
            format!("{} {}", self.bin, join_args(args))
        };

        (path::replace_home_dir(line), cmd.get_current_dir())
    }

    pub fn get_input_line(&self) -> String {
        String::from_utf8(self.input.clone())
                .unwrap_or_default()
                .replace('\n', " ")
    }

    pub fn inherit_colors(&mut self) -> &mut Command {
        let level = color::supports_color().to_string();

        self.env("FORCE_COLOR", &level);
        self.env("CLICOLOR_FORCE", &level);

        // Force a terminal width so that we have consistent sizing
        // in our cached output, and its the same across all machines
        // https://help.gnome.org/users/gnome-terminal/stable/app-terminal-sizes.html.en
        self.env("COLUMNS", "80");
        self.env("LINES", "24");

        self
    }

    pub fn no_error_on_failure(&mut self) -> &mut Command {
        self.error_on_nonzero = false;
        self
    }

    pub fn set_prefix(&mut self, prefix: &str, width: Option<usize>) -> &mut Command {
        if is_ci() && !is_test_env() {
            self.prefix = Some(color::muted(format!("[{}]", prefix)));
        } else {
            self.prefix = Some(format!(
                "{} {}",
                color::log_target(if let Some(width) = width {
                    pad_str(prefix, width, Alignment::Left, None).to_string()
                } else {
                    prefix.to_owned()
                }),
                color::muted("|")
            ));
        }

        self
    }

    pub fn output_to_error(&self, output: &Output, with_message: bool) -> MoonError {
        let code = output.status.code().unwrap_or(-1);

        if !with_message {
            return MoonError::ProcessNonZero(self.bin.clone(), code);
        }

        let mut message = output_to_trimmed_string(&output.stderr);

        if message.is_empty() {
            message = output_to_trimmed_string(&output.stdout);
        }

        MoonError::ProcessNonZeroWithOutput(self.bin.clone(), code, message)
    }

    fn handle_nonzero_status(&self, output: &Output) -> Result<(), MoonError> {
        if self.error_on_nonzero && !output.status.success() {
            return Err(self.output_to_error(output, true));
        }

        Ok(())
    }

    #[track_caller]
    fn log_command_info(&self) {
        // Avoid all this overhead if we're not logging
        if !logging_enabled() {
            return;
        }

        let cmd = &self.cmd.as_std();
        let (mut command_line, working_dir) = self.get_command_line();

        if !self.input.is_empty() {
            if command_line.ends_with("-") {
                command_line = format!("{} {}", command_line, self.get_input_line());
            } else {
                command_line = format!("{} - {}", command_line, self.get_input_line());
            }
        }

        let mut envs_list = vec![];

        for (key, value) in cmd.get_envs() {
            if value.is_some() {
                let key_str = key.to_str().unwrap_or_default();

                if key_str.starts_with("MOON_") || key_str.starts_with("NODE_") {
                    envs_list.push(format!(
                        "\n  {}{}{}",
                        key_str,
                        color::muted("="),
                        color::muted_light(value.unwrap().to_str().unwrap())
                    ));
                }
            }
        }

        trace!(
            target: "moon:utils:process",
            "Running command {} (in {}){}",
            color::shell(&command_line),
            if let Some(cwd) = working_dir {
                color::path(cwd)
            } else {
                String::from("working dir")
            },
            envs_list.join("")
        );
    }

    async fn write_input_to_child(&self, child: &mut Child) -> Result<(), MoonError> {
        let mut stdin = child.stdin.take().unwrap_or_else(|| {
            panic!("Unable to write stdin: {}", self.get_input_line());
        });

        stdin.write_all(&self.input).await?;

        drop(stdin);

        Ok(())
    }
}
