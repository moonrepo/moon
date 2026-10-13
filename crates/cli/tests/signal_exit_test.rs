//! A running task interrupted by SIGINT or SIGTERM must not exit 0, and the
//! summary must not count that task as skipped.
//!
//! Before the fix this exits 0 and prints "skipped".

#![cfg(unix)]

use moon_test_utils::create_empty_moon_sandbox;
use std::io::Read;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

fn pump(
    mut reader: impl Read + Send + 'static,
    output: Arc<Mutex<String>>,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let mut chunk = [0u8; 256];

        loop {
            match reader.read(&mut chunk) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    let text = String::from_utf8_lossy(&chunk[..n]);

                    if let Ok(mut buf) = output.lock() {
                        buf.push_str(&text);
                    }
                }
            }
        }
    })
}

fn interrupt_running_task(signal: &str, expected: i32) {
    let sandbox = create_empty_moon_sandbox();
    let root = sandbox.path().to_path_buf();

    sandbox.enable_git();
    sandbox.create_file(".moon/workspace.yml", "projects:\n  - \"app\"\n");
    sandbox.create_file(
        "app/moon.yml",
        "\
tasks:
  wait:
    command: \"sleep 30\"
    options:
      cache: false
",
    );

    sandbox.run_git(|cmd| {
        cmd.args(["config", "user.email", "fakeemail@somedomain.dev"]);
    });
    sandbox.run_git(|cmd| {
        cmd.args(["config", "user.name", "Sandbox"]);
    });
    sandbox.run_git(|cmd| {
        cmd.args(["add", "-A"]);
    });
    sandbox.run_git(|cmd| {
        cmd.args(["commit", "-qm", "init"]);
    });

    let mut child = Command::new(env!("CARGO_BIN_EXE_moon"))
        .args(["run", "app:wait"])
        .current_dir(&root)
        .env("MOON_TEST", "true")
        .env("STARBASE_TEST", "true")
        .env("NO_COLOR", "1")
        .env("MOON_HOME", root.join(".moon"))
        .env("PROTO_HOME", root.join(".proto"))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn moon");

    let pid = child.id();
    let output = Arc::new(Mutex::new(String::new()));
    let stdout_thread = pump(child.stdout.take().expect("stdout"), Arc::clone(&output));
    let stderr_thread = pump(child.stderr.take().expect("stderr"), Arc::clone(&output));

    // Wait until the task line is printed, so the signal lands while the
    // 30s command is running, not during startup on a slow runner.
    let deadline = Instant::now() + Duration::from_secs(60);

    while !output.lock().is_ok_and(|buf| buf.contains("app:wait")) {
        if Instant::now() > deadline {
            let _ = child.kill();

            panic!(
                "task app:wait did not start within 60s:\n{}",
                output.lock().map(|buf| buf.clone()).unwrap_or_default()
            );
        }

        std::thread::sleep(Duration::from_millis(100));
    }

    // Give the task a moment to spawn `sleep` after its line is printed.
    std::thread::sleep(Duration::from_secs(1));

    let killed = Command::new("kill")
        .args([signal, &pid.to_string()])
        .status()
        .expect("kill");
    assert!(killed.success(), "failed to send {signal} to {pid}");

    let (cancel_tx, cancel_rx) = mpsc::channel::<()>();
    let watchdog = std::thread::spawn(move || {
        if cancel_rx.recv_timeout(Duration::from_secs(15)).is_err() {
            let _ = Command::new("kill")
                .args(["-KILL", &pid.to_string()])
                .status();
        }
    });

    let status = child.wait().expect("wait for moon");
    let _ = cancel_tx.send(());
    let _ = watchdog.join();
    let _ = stdout_thread.join();
    let _ = stderr_thread.join();

    let output = output.lock().map(|buf| buf.clone()).unwrap_or_default();

    assert_eq!(
        status.code(),
        Some(expected),
        "signal {signal} should exit {expected}, got {status:?}\n{output}"
    );
    assert!(
        !output.to_ascii_lowercase().contains("skipped"),
        "summary counted the interrupted task as skipped:\n{output}"
    );
    assert!(
        output.to_ascii_lowercase().contains("failed"),
        "summary should count the interrupted task as failed:\n{output}"
    );
}

#[test]
fn sigint_exits_130_and_is_not_skipped() {
    interrupt_running_task("-INT", 130);
}

#[test]
fn sigterm_exits_143_and_is_not_skipped() {
    interrupt_running_task("-TERM", 143);
}
