use crate::path;
use moon_error::{map_io_to_process_error, MoonError};
use moon_logger::{color, logging_enabled, trace};
use std::env;
use std::ffi::OsStr;
use std::path::Path;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command as TokioCommand;
use tokio::sync::RwLock;
use tokio::task;

pub use std::process::{ExitStatus, Output, Stdio};

#[cfg(not(windows))]
pub fn create_command<S: AsRef<OsStr>>(bin: S) -> TokioCommand {
    TokioCommand::new(bin)
}

#[cfg(windows)]
pub fn create_command<S: AsRef<OsStr>>(bin: S) -> TokioCommand {
    let bin_name = bin.as_ref().to_str().unwrap_or_default();

    // Based on how Node.js executes Windows commands:
    // https://github.com/nodejs/node/blob/master/lib/child_process.js#L572
    if bin_name.ends_with(".cmd") || bin_name.ends_with(".bat") {
        let mut cmd = TokioCommand::new("cmd.exe");
        cmd.arg("/d");
        cmd.arg("/s");
        cmd.arg("/c");
        cmd.arg(bin);
        cmd
    } else {
        TokioCommand::new(bin)
    }
}

pub fn output_to_string(data: &[u8]) -> String {
    String::from_utf8(data.to_vec()).unwrap_or_default()
}

pub fn output_to_trimmed_string(data: &[u8]) -> String {
    output_to_string(data).trim().to_owned()
}

pub struct Command {
    bin: String,
    cmd: TokioCommand,
}

// This is rather annoying that we have to re-implement all these methods,
// but the encapsulation this struct provides is necessary.
impl Command {
    pub fn new<S: AsRef<OsStr>>(bin: S) -> Self {
        Command {
            bin: String::from(bin.as_ref().to_string_lossy()),
            cmd: create_command(bin),
        }
    }

    pub fn arg<S: AsRef<OsStr>>(&mut self, arg: S) -> &mut Command {
        self.cmd.arg(arg);
        self
    }

    pub fn args<I, S>(&mut self, args: I) -> &mut Command
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        self.cmd.args(args);
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

    pub async fn exec_capture_output(&mut self) -> Result<Output, MoonError> {
        self.log_command_info();

        let output = self.cmd.output();
        let output = output
            .await
            .map_err(|e| map_io_to_process_error(e, &self.bin))?;

        self.handle_nonzero_status(&output.status)?;

        Ok(output)
    }

    pub async fn exec_capture_output_with_input(
        &mut self,
        input: &str,
    ) -> Result<Output, MoonError> {
        self.log_command_info();

        let mut child = self
            .cmd
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| map_io_to_process_error(e, &self.bin))?;

        let mut stdin = child.stdin.take().unwrap();
        stdin.write_all(input.as_bytes()).await.unwrap();
        drop(stdin);

        let output = child
            .wait_with_output()
            .await
            .map_err(|e| map_io_to_process_error(e, &self.bin))?;

        self.handle_nonzero_status(&output.status)?;

        Ok(output)
    }

    pub async fn exec_stream_output(&mut self) -> Result<ExitStatus, MoonError> {
        self.log_command_info();

        let status = self
            .cmd
            .spawn()
            .map_err(|e| map_io_to_process_error(e, &self.bin))?
            .wait()
            .await
            .map_err(|e| map_io_to_process_error(e, &self.bin))?;

        self.handle_nonzero_status(&status)?;

        Ok(status)
    }

    pub async fn exec_stream_and_capture_output(&mut self) -> Result<Output, MoonError> {
        self.log_command_info();

        let mut child = self
            .cmd
            .stderr(Stdio::piped())
            .stdout(Stdio::piped())
            .envs(env::vars())
            // Inherit ANSI colors since they're stripped from pipes
            .env("FORCE_COLOR", env::var("FORCE_COLOR").unwrap_or_default())
            .env("TERM", env::var("TERM").unwrap_or_default())
            .spawn()
            .map_err(|e| map_io_to_process_error(e, &self.bin))?;

        // We need to log the child process output to the parent terminal
        // AND capture stdout/stderr so that we can cache it for future runs.
        // This doesn't seem to be supported natively by `Stdio`, so I have
        // this *real ugly* implementation to solve it. There's gotta be a
        // better way to do this?
        // https://stackoverflow.com/a/49063262
        let err = BufReader::new(child.stderr.take().unwrap());
        let out = BufReader::new(child.stdout.take().unwrap());

        // Spawn additional threads for logging the buffer
        let stderr = Arc::new(RwLock::new(vec![]));
        let stdout = Arc::new(RwLock::new(vec![]));
        let stderr_clone = Arc::clone(&stderr);
        let stdout_clone = Arc::clone(&stdout);

        task::spawn(async move {
            let mut lines = err.lines();
            let mut stderr_write = stderr_clone.write().await;

            while let Some(line) = lines.next_line().await.unwrap() {
                eprintln!("{}", line);
                stderr_write.push(line);
            }
        });

        task::spawn(async move {
            let mut lines = out.lines();
            let mut stdout_write = stdout_clone.write().await;

            while let Some(line) = lines.next_line().await.unwrap() {
                println!("{}", line);
                stdout_write.push(line);
            }
        });

        // Attempt to capture the child output
        let mut output = child
            .wait_with_output()
            .await
            .map_err(|e| map_io_to_process_error(e, &self.bin))?;

        if output.stderr.is_empty() {
            output.stderr = stderr.read().await.join("").into_bytes();
        }

        if output.stdout.is_empty() {
            output.stdout = stdout.read().await.join("").into_bytes();
        }

        self.handle_nonzero_status(&output.status)?;

        Ok(output)
    }

    fn handle_nonzero_status(&self, status: &ExitStatus) -> Result<(), MoonError> {
        if !status.success() {
            match status.code() {
                Some(code) => {
                    return Err(MoonError::ProcessNonZero(self.bin.clone(), code));
                }
                None => return Err(MoonError::ProcessNonZero(self.bin.clone(), -1)),
            };
        }

        Ok(())
    }

    fn log_command_info(&self) {
        // Avoid all this overhead if we're not logging
        if !logging_enabled() {
            return;
        }

        let cmd = &self.cmd.as_std();
        let args = cmd
            .get_args()
            .into_iter()
            .map(|a| a.to_str().unwrap())
            .collect::<Vec<_>>();
        let command_line = path::replace_home_dir(&format!("{} {}", self.bin, args.join(" ")));

        if let Some(cwd) = cmd.get_current_dir() {
            trace!(
                target: "moon:utils",
                "Running command {} (in {})",
                color::shell(&command_line),
                color::path(cwd),
            );
        } else {
            trace!(
                target: "moon:utils",
                "Running command {} ",
                color::shell(&command_line),
            );
        }
    }
}
