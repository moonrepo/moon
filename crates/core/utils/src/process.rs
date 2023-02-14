use crate::{get_workspace_root, is_ci, is_test_env, path, shell};
use moon_error::{map_io_to_process_error, MoonError};
use moon_logger::{color, logging_enabled, pad_str, trace, Alignment};
use rustc_hash::FxHashMap;
use std::env;
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command as TokioCommand};
use tokio::task;

pub use shell_words::{join as join_args, split as split_args, ParseError as ArgsParseError};
pub use std::process::{ExitStatus, Output, Stdio};

#[inline]
pub fn is_windows_script(bin: &str) -> bool {
    bin.ends_with(".cmd") || bin.ends_with(".bat") || bin.ends_with(".ps1")
}

#[inline]
pub fn output_to_string(data: &[u8]) -> String {
    String::from_utf8(data.to_vec()).unwrap_or_default()
}

#[inline]
pub fn output_to_trimmed_string(data: &[u8]) -> String {
    output_to_string(data).trim().to_owned()
}

#[inline]
pub fn format_running_command(
    command_line: &str,
    working_dir: Option<&Path>,
    workspace_root: Option<&Path>,
) -> String {
    let workspace_root = match workspace_root {
        Some(root) => root.to_owned(),
        None => get_workspace_root(),
    };

    let working_dir = working_dir.unwrap_or(&workspace_root);

    let target_dir = if working_dir == workspace_root {
        String::from("workspace")
    } else {
        format!(
            ".{}{}",
            std::path::MAIN_SEPARATOR,
            working_dir
                .strip_prefix(&workspace_root)
                .unwrap()
                .to_string_lossy(),
        )
    };

    let suffix = format!("(in {target_dir})");
    let message = format!("{} {}", command_line, color::muted(suffix));

    color::muted_light(message)
}

#[derive(Debug)]
pub struct Command {
    args: Vec<OsString>,

    bin: String,

    cwd: Option<PathBuf>,

    env: FxHashMap<OsString, OsString>,

    /// Convert non-zero exits to errors
    error_on_nonzero: bool,

    /// Values to pass to stdin
    input: Vec<OsString>,

    /// Log the command to the terminal before running
    log_command: bool,

    /// Arguments will be passed via stdin to the command
    pass_args_stdin: bool,

    /// Prefix to prepend to all log lines
    prefix: Option<String>,

    /// Shell to wrap executing commands in
    shell: Option<shell::Shell>,
}

// This is rather annoying that we have to re-implement all these methods,
// but the encapsulation this struct provides is necessary.
impl Command {
    pub fn new<S: AsRef<OsStr>>(bin: S) -> Self {
        let bin = bin.as_ref();

        let mut command = Command {
            bin: bin.to_string_lossy().to_string(),
            args: vec![],
            cwd: None,
            env: FxHashMap::default(),
            error_on_nonzero: true,
            input: vec![],
            log_command: false,
            pass_args_stdin: false,
            prefix: None,
            shell: None,
        };

        // Referencing a batch script needs to be ran with a shell
        if is_windows_script(&command.bin) {
            command.pass_args_stdin = true;
            command.shell = Some(shell::create_windows_shell());
        }

        command
    }

    pub fn arg<S: AsRef<OsStr>>(&mut self, arg: S) -> &mut Command {
        self.args.push(arg.as_ref().to_os_string());
        self
    }

    pub fn arg_if_missing<S: AsRef<OsStr>>(&mut self, arg: S) -> &mut Command {
        let arg = arg.as_ref();
        let present = self.args.iter().any(|a| a == arg);

        if !present {
            self.arg(arg);
        }

        self
    }

    pub fn args<I, S>(&mut self, args: I) -> &mut Command
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        for arg in args {
            self.arg(arg);
        }

        self
    }

    pub fn cwd<P: AsRef<Path>>(&mut self, dir: P) -> &mut Command {
        self.cwd = Some(dir.as_ref().to_path_buf());
        self
    }

    pub fn env<K, V>(&mut self, key: K, val: V) -> &mut Command
    where
        K: AsRef<OsStr>,
        V: AsRef<OsStr>,
    {
        self.env
            .insert(key.as_ref().to_os_string(), val.as_ref().to_os_string());
        self
    }

    pub fn envs<I, K, V>(&mut self, vars: I) -> &mut Command
    where
        I: IntoIterator<Item = (K, V)>,
        K: AsRef<OsStr>,
        V: AsRef<OsStr>,
    {
        for (k, v) in vars {
            self.env(k, v);
        }

        self
    }

    pub fn input<I, V>(&mut self, input: I) -> &mut Command
    where
        I: IntoIterator<Item = V>,
        V: AsRef<OsStr>,
    {
        for i in input {
            self.input.push(i.as_ref().to_os_string());
        }

        self
    }

    pub async fn exec_capture_output(&mut self) -> Result<Output, MoonError> {
        self.log_command_info();

        let mut command = self.get_command();
        let error_handler = |e| map_io_to_process_error(e, &self.bin);
        let output: Output;

        if !self.has_input() {
            output = command.output().await.map_err(error_handler)?;
        } else {
            let mut child = command
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

    pub async fn exec_stream_output(&mut self) -> Result<Output, MoonError> {
        self.log_command_info();

        let mut command = self.get_command();
        let error_handler = |e| map_io_to_process_error(e, &self.bin);
        let mut child: Child;

        if !self.has_input() {
            child = command.spawn().map_err(error_handler)?;
        } else {
            child = command
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

        let output = Output {
            status,
            stderr: vec![],
            stdout: vec![],
        };

        Ok(output)
    }

    pub async fn exec_stream_and_capture_output(&mut self) -> Result<Output, MoonError> {
        self.log_command_info();

        let mut command = self.get_command();
        let has_input = self.has_input();
        let error_handler = |e| map_io_to_process_error(e, &self.bin);

        let mut child = command
            .stdin(if has_input {
                Stdio::piped()
            } else {
                Stdio::inherit()
            })
            .stderr(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .map_err(error_handler)?;

        if has_input {
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

            while let Ok(Some(line)) = lines.next_line().await {
                if stderr_prefix.is_empty() {
                    eprintln!("{line}");
                } else {
                    eprintln!("{stderr_prefix} {line}");
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

            while let Ok(Some(line)) = lines.next_line().await {
                if stdout_prefix.is_empty() {
                    println!("{line}");
                } else {
                    println!("{stdout_prefix} {line}");
                }

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
        let status = child.wait().await.map_err(error_handler)?;

        let output = Output {
            status,
            stdout: captured_stdout.read().unwrap().join("\n").into_bytes(),
            stderr: captured_stderr.read().unwrap().join("\n").into_bytes(),
        };

        self.handle_nonzero_status(&output)?;

        Ok(output)
    }

    pub fn get_command(&mut self) -> TokioCommand {
        let mut command = if let Some(shell) = &self.shell {
            let mut cmd = TokioCommand::new(&shell.command);
            cmd.args(&shell.args);

            if !self.pass_args_stdin {
                cmd.arg(&self.bin);
                cmd.args(&self.args);
            }

            cmd
        } else {
            let mut cmd = TokioCommand::new(&self.bin);
            cmd.args(&self.args);
            cmd
        };

        if let Some(cwd) = &self.cwd {
            command.current_dir(cwd);
        }

        // Avoid zombie processes, especially for long-running
        // or never-ending tasks!
        command.kill_on_drop(true);

        command.envs(&self.env);
        command
    }

    pub fn get_command_line(&self) -> (String, Option<&Path>) {
        let line = if self.args.is_empty() {
            self.bin.to_owned()
        } else {
            format!(
                "{} {}",
                &self.bin,
                join_args(
                    self.args
                        .iter()
                        .map(|a| a.to_str().unwrap_or("<unknown>"))
                        .collect::<Vec<_>>()
                )
            )
        };

        (path::replace_home_dir(line), self.cwd.as_deref())
    }

    pub fn get_input_line(&self) -> String {
        let mut input: Vec<&OsString> = vec![];
        let bin = OsString::from(&self.bin);

        // When no input, inherit the arguments and the binary to execute
        if self.input.is_empty() {
            input.push(&bin);
            input.extend(&self.args);
        } else {
            input.extend(&self.input);
        }

        input
            .iter()
            .map(|i| i.as_os_str())
            .collect::<Vec<_>>()
            .join(OsStr::new(" "))
            .to_str()
            .unwrap_or_default()
            .to_string()
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

    pub fn log_running_command(&mut self, state: bool) -> &mut Command {
        self.log_command = state;
        self
    }

    pub fn no_error_on_failure(&mut self) -> &mut Command {
        self.error_on_nonzero = false;
        self
    }

    pub fn set_prefix(&mut self, prefix: &str, width: Option<usize>) -> &mut Command {
        if is_ci() && !is_test_env() {
            self.prefix = Some(color::muted(format!("[{prefix}]")));
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

    fn has_input(&self) -> bool {
        !self.input.is_empty() || self.pass_args_stdin
    }

    #[track_caller]
    fn log_command_info(&self) {
        let (mut command_line, working_dir) = self.get_command_line();

        if self.log_command {
            println!(
                "{}",
                format_running_command(&command_line, working_dir, None)
            );
        }

        // Avoid all this overhead if we're not logging
        if !logging_enabled() {
            return;
        }

        if self.has_input() {
            let input_line = self.get_input_line();
            let debug_input = env::var("MOON_DEBUG_PROCESS_INPUT").is_ok();

            command_line = format!(
                "{}{}{}",
                command_line,
                if command_line.ends_with('-') {
                    " "
                } else {
                    " - "
                },
                if input_line.len() > 200 && !debug_input {
                    "(truncated files list)".into()
                } else {
                    input_line.replace('\n', " ")
                }
            );
        }

        let mut envs_list = vec![];

        for (key, value) in &self.env {
            let key = key.to_str().unwrap_or_default();

            if key.starts_with("MOON_") || key.starts_with("PROTO_") || key.starts_with("NODE_") {
                envs_list.push(format!(
                    "\n  {}{}{}",
                    key,
                    color::muted("="),
                    color::muted_light(value.to_str().unwrap())
                ));
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
        let input = self.get_input_line();

        let mut stdin = child.stdin.take().unwrap_or_else(|| {
            panic!("Unable to write stdin: {input}");
        });

        stdin.write_all(input.as_bytes()).await?;

        drop(stdin);

        Ok(())
    }
}
