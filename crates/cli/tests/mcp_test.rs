//! `moon mcp` is a stdio MCP server, so it must exit once its client closes
//! stdin, which is how a client shuts down the server, even a client that
//! was killed and never sent a signal.
//!
//! Before the fix the server kept running until it received a signal.

use moon_test_utils::{MoonSandbox, create_empty_moon_sandbox};
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, ExitStatus, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

const TIMEOUT: Duration = Duration::from_secs(60);

fn create_workspace() -> MoonSandbox {
    let sandbox = create_empty_moon_sandbox();
    sandbox.create_file(".moon/workspace.yml", "projects:\n  - \"app\"\n");
    sandbox.create_file(
        "app/moon.yml",
        "tasks:\n  build:\n    command: \"echo build\"\n",
    );
    sandbox.enable_git();
    sandbox
}

/// Write stderr to a file instead of a pipe, so that reading it can't
/// block on a process that inherited the pipe and is still running.
fn log_path(sandbox: &MoonSandbox) -> PathBuf {
    sandbox.path().join("mcp.log")
}

fn read_log(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_default()
}

fn spawn_mcp(sandbox: &MoonSandbox, stdin: Stdio, stdout: Stdio) -> Child {
    let log = File::create(log_path(sandbox)).expect("create log file");

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_moon"));
    cmd.arg("mcp")
        .current_dir(sandbox.path())
        .stdin(stdin)
        .stdout(stdout)
        .stderr(Stdio::from(log));

    for (key, value) in &sandbox.settings.env {
        cmd.env(key, value);
    }

    cmd.spawn().expect("spawn moon mcp")
}

fn kill(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

fn wait_for_exit(child: &mut Child, log: &Path, reason: &str) -> ExitStatus {
    let deadline = Instant::now() + TIMEOUT;

    loop {
        if let Some(status) = child.try_wait().expect("wait for moon mcp") {
            return status;
        }

        if Instant::now() > deadline {
            kill(child);

            panic!(
                "moon mcp did not exit within {}s after {reason}:\n{}",
                TIMEOUT.as_secs(),
                read_log(log)
            );
        }

        thread::sleep(Duration::from_millis(100));
    }
}

/// Send a `tools/list` request and wait for its response, which proves the
/// server is up and serving requests. Returns stdin, which is still open.
fn list_tools(child: &mut Child, log: &Path) -> ChildStdin {
    let mut stdin = child.stdin.take().expect("stdin");
    let mut stdout = BufReader::new(child.stdout.take().expect("stdout"));

    stdin
        .write_all(
            concat!(
                r#"{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{"_meta":{"#,
                r#""io.modelcontextprotocol/protocolVersion":"2026-07-28","#,
                r#""io.modelcontextprotocol/clientCapabilities":{}}}}"#,
                "\n"
            )
            .as_bytes(),
        )
        .expect("write request");
    stdin.flush().expect("flush request");

    // Read in a thread, so that a server that never responds fails the test
    // instead of hanging it
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        let mut line = String::new();
        let _ = stdout.read_line(&mut line);
        let _ = tx.send(line);
    });

    let Ok(response) = rx.recv_timeout(TIMEOUT) else {
        kill(child);

        panic!(
            "moon mcp did not respond to tools/list within {}s:\n{}",
            TIMEOUT.as_secs(),
            read_log(log)
        );
    };

    assert!(
        response.contains(r#""result""#) && response.contains("get_projects"),
        "tools/list should list the tools, got: {response}\n{}",
        read_log(log)
    );

    stdin
}

#[test]
fn exits_when_stdin_is_already_closed() {
    let sandbox = create_workspace();
    let log = log_path(&sandbox);
    let mut child = spawn_mcp(&sandbox, Stdio::null(), Stdio::null());

    let status = wait_for_exit(&mut child, &log, "stdin was closed");

    assert!(
        status.success(),
        "moon mcp should exit 0 when stdin is closed, got {status:?}\n{}",
        read_log(&log)
    );
}

#[test]
fn exits_when_client_closes_stdin_after_a_request() {
    let sandbox = create_workspace();
    let log = log_path(&sandbox);
    let mut child = spawn_mcp(&sandbox, Stdio::piped(), Stdio::piped());

    let stdin = list_tools(&mut child, &log);

    // The client goes away
    drop(stdin);

    let status = wait_for_exit(&mut child, &log, "the client closed stdin");

    assert!(
        status.success(),
        "moon mcp should exit 0 when the client closes stdin, got {status:?}\n{}",
        read_log(&log)
    );
}

#[cfg(unix)]
#[test]
fn exits_on_signal_while_stdin_is_open() {
    let sandbox = create_workspace();
    let log = log_path(&sandbox);
    let mut child = spawn_mcp(&sandbox, Stdio::piped(), Stdio::piped());

    // Keep stdin open, so that only the signal can stop the server
    let _stdin = list_tools(&mut child, &log);

    let killed = Command::new("kill")
        .args(["-TERM", &child.id().to_string()])
        .status()
        .expect("kill");
    assert!(killed.success(), "failed to send SIGTERM to moon mcp");

    let status = wait_for_exit(&mut child, &log, "SIGTERM");

    assert!(
        status.success(),
        "moon mcp should exit 0 on SIGTERM, got {status:?}\n{}",
        read_log(&log)
    );
}
