use crate::fs::get_home_dir;
use moon_error::{map_io_to_process_error, MoonError};
use moon_logger::{color, logging_enabled, trace};
use std::env;
use std::ffi::OsStr;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::RwLock;
use tokio::task;

pub use std::process::{Output, Stdio};

fn log_command_info(command: &Command) {
    // Avoid all this overhead if we're not logging
    if !logging_enabled() {
        return;
    }

    let cmd = command.as_std();
    let bin_name = cmd.get_program().to_str().unwrap_or("<unknown>");
    let args_list = cmd
        .get_args()
        .into_iter()
        .map(|a| a.to_str().unwrap())
        .collect::<Vec<_>>();
    let command_line = format!("{} {}", bin_name, args_list.join(" "))
        .replace(get_home_dir().unwrap_or_default().to_str().unwrap(), "~");

    if let Some(cwd) = cmd.get_current_dir() {
        trace!(
            target: "moon:utils",
            "Running command {} (in {})",
            color::shell(&command_line),
            color::file_path(cwd),
        );
    } else {
        trace!(
            target: "moon:utils",
            "Running command {} ",
            color::shell(&command_line),
        );
    }
}

#[cfg(not(windows))]
pub fn create_command<S: AsRef<OsStr>>(bin: S) -> Command {
    Command::new(bin)
}

#[cfg(windows)]
pub fn create_command<S: AsRef<OsStr>>(bin: S) -> Command {
    let bin_name = bin.as_ref().to_str().unwrap_or_default();

    // Based on how Node.js executes Windows commands:
    // https://github.com/nodejs/node/blob/master/lib/child_process.js#L572
    if bin_name.ends_with(".cmd") || bin_name.ends_with(".bat") {
        let mut cmd = Command::new("cmd.exe");
        cmd.arg("/d");
        cmd.arg("/s");
        cmd.arg("/c");
        cmd.arg(bin);
        cmd
    } else {
        Command::new(bin)
    }
}

pub async fn exec_command(command: &mut Command) -> Result<Output, MoonError> {
    log_command_info(command);

    let output = command.output();
    let output = output.await.map_err(|e| {
        map_io_to_process_error(e, command.as_std().get_program().to_str().unwrap())
    })?;

    handle_nonzero_status(command, &output)?;

    Ok(output)
}

pub async fn exec_command_capture_stderr(command: &mut Command) -> Result<String, MoonError> {
    let output = exec_command(command).await?;

    Ok(output_to_string(&output.stderr))
}

pub async fn exec_command_capture_stdout(command: &mut Command) -> Result<String, MoonError> {
    let output = exec_command(command).await?;

    Ok(output_to_string(&output.stdout))
}

pub async fn spawn_command(command: &mut Command) -> Result<Output, MoonError> {
    log_command_info(command);

    let mut child = command
        .stderr(Stdio::piped())
        .stdout(Stdio::piped())
        .envs(env::vars())
        // Inherit ANSI colors since they're stripped from pipes
        .env("FORCE_COLOR", env::var("FORCE_COLOR").unwrap_or_default())
        .env("TERM", env::var("TERM").unwrap_or_default())
        .spawn()
        .unwrap();

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
    let mut output = child.wait_with_output().await.map_err(|e| {
        map_io_to_process_error(e, command.as_std().get_program().to_str().unwrap())
    })?;

    if output.stderr.is_empty() {
        output.stderr = stderr.read().await.join("").into_bytes();
    }

    if output.stdout.is_empty() {
        output.stdout = stdout.read().await.join("").into_bytes();
    }

    handle_nonzero_status(command, &output)?;

    Ok(output)
}

pub fn output_to_string(data: &[u8]) -> String {
    String::from_utf8(data.to_vec()).unwrap_or_default()
}

fn handle_nonzero_status(command: &mut Command, output: &Output) -> Result<(), MoonError> {
    if !output.status.success() {
        let bin_name = command
            .as_std()
            .get_program()
            .to_str()
            .unwrap_or("<unknown>");

        match output.status.code() {
            Some(code) => {
                return Err(MoonError::ProcessNonZero(
                    bin_name.to_owned(),
                    code,
                    output_to_string(&output.stderr), // Always correct?
                ));
            }
            None => {
                return Err(MoonError::ProcessNonZero(
                    bin_name.to_owned(),
                    -1,
                    String::from("Process terminated by signal."),
                ))
            }
        };
    }

    Ok(())
}
