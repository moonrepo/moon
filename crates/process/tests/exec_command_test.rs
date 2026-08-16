#![cfg(unix)]

use moon_console::Console;
use moon_process::{CaptureOptions, ChildExit, Command, ProcessError};
use std::sync::Arc;
use std::time::Duration;

fn create_command(script: &str) -> Command {
    let mut command = Command::new("bash");
    command.args(["-c", script]);
    command.no_shell();
    command.set_console(Arc::new(Console::new_testing()));
    command
}

mod exec_capture_output {
    use super::*;

    #[tokio::test]
    async fn captures_stdout_and_stderr() {
        let output = create_command("printf 'out'; printf 'err' 1>&2")
            .exec_capture_output()
            .await
            .unwrap();

        assert!(output.success());
        assert_eq!(output.stdout.as_ref(), b"out");
        assert_eq!(output.stderr.as_ref(), b"err");
    }

    #[tokio::test]
    async fn errors_on_nonzero_exit() {
        let error = create_command("echo 'oops' 1>&2; exit 3")
            .exec_capture_output()
            .await
            .unwrap_err();

        match error.downcast_ref::<ProcessError>().unwrap() {
            ProcessError::ExitNonZeroWithOutput { status, output, .. } => {
                assert_eq!(status, "exit code 3");
                assert!(output.contains("oops"));
            }
            _ => panic!("expected ExitNonZeroWithOutput"),
        };
    }

    #[tokio::test]
    async fn can_allow_nonzero_exit() {
        let mut command = create_command("exit 3");
        command.set_error_on_nonzero(false);

        let output = command.exec_capture_output().await.unwrap();

        assert!(!output.success());
        assert_eq!(output.code(), Some(3));
    }

    #[tokio::test]
    async fn passes_input_to_stdin() {
        let mut command = create_command("cat");
        command.input(["hello", "world"]);

        let output = command.exec_capture_output().await.unwrap();

        assert_eq!(output.stdout.as_ref(), b"hello world");
    }

    #[tokio::test]
    async fn survives_child_exiting_before_consuming_stdin() {
        // The child exits without reading stdin while we write input far
        // larger than any pipe buffer, so the writer hits a broken pipe.
        // That must be benign: the child's exit status is the outcome.
        let mut command = create_command("exit 0");
        command.input(vec!["x".repeat(1024); 2048]);

        let output = command.exec_capture_output().await.unwrap();

        assert!(output.success());
    }

    #[tokio::test]
    async fn reports_killed_children() {
        let mut command = create_command("kill -9 $$");
        command.set_error_on_nonzero(false);

        let output = command.exec_capture_output().await.unwrap();

        assert!(!output.success());
        assert_eq!(output.exit, ChildExit::Killed);
    }
}

mod exec_capture_output_blocking {
    use super::*;
    use std::fs::File;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn create_output_files() -> (std::path::PathBuf, File, std::path::PathBuf, File) {
        static NEXT_ID: AtomicUsize = AtomicUsize::new(0);

        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        let prefix = format!("moon-process-{}-{id}", std::process::id());
        let stdout_path = std::env::temp_dir().join(format!("{prefix}-stdout"));
        let stderr_path = std::env::temp_dir().join(format!("{prefix}-stderr"));
        let stdout = File::create(&stdout_path).unwrap();
        let stderr = File::create(&stderr_path).unwrap();

        (stdout_path, stdout, stderr_path, stderr)
    }

    #[test]
    fn captures_stdout_and_stderr() {
        let output = create_command("printf 'out'; printf 'err' 1>&2")
            .exec_capture_output_blocking(&CaptureOptions::default())
            .unwrap();

        assert!(output.success());
        assert_eq!(output.stdout.as_ref(), b"out");
        assert_eq!(output.stderr.as_ref(), b"err");
    }

    #[test]
    fn passes_input_to_stdin() {
        let mut command = create_command("cat");
        command.input(["hello", "world"]);

        let output = command
            .exec_capture_output_blocking(&CaptureOptions::default())
            .unwrap();

        assert_eq!(output.stdout.as_ref(), b"hello world");
    }

    #[test]
    fn can_allow_nonzero_exit() {
        let mut command = create_command("exit 3");
        command.set_error_on_nonzero(false);

        let output = command
            .exec_capture_output_blocking(&CaptureOptions::default())
            .unwrap();

        assert!(!output.success());
        assert_eq!(output.code(), Some(3));
    }

    #[test]
    fn enforces_timeout() {
        let error = create_command("sleep 30")
            .exec_capture_output_blocking(&CaptureOptions {
                timeout: Some(Duration::from_millis(50)),
                ..CaptureOptions::default()
            })
            .unwrap_err();

        assert!(matches!(
            error.downcast_ref::<ProcessError>(),
            Some(ProcessError::Timeout { .. })
        ));
    }

    #[test]
    fn enforces_combined_output_limit() {
        let error = create_command("printf '12345'; printf '67890' 1>&2")
            .exec_capture_output_blocking(&CaptureOptions {
                output_limit: Some(8),
                ..CaptureOptions::default()
            })
            .unwrap_err();

        assert!(matches!(
            error.downcast_ref::<ProcessError>(),
            Some(ProcessError::OutputLimitExceeded { limit: 8, .. })
        ));
    }

    #[test]
    fn never_returns_truncated_output_at_the_limit() {
        for _ in 0..100 {
            let result = create_command("printf '123456789'").exec_capture_output_blocking(
                &CaptureOptions {
                    output_limit: Some(8),
                    ..CaptureOptions::default()
                },
            );

            assert!(matches!(
                result.unwrap_err().downcast_ref::<ProcessError>(),
                Some(ProcessError::OutputLimitExceeded { limit: 8, .. })
            ));
        }
    }

    #[test]
    fn timeout_terminates_descendants_holding_output_pipes() {
        let error = create_command("sleep 30 & printf 'ready'")
            .exec_capture_output_blocking(&CaptureOptions {
                timeout: Some(Duration::from_millis(50)),
                ..CaptureOptions::default()
            })
            .unwrap_err();

        assert!(matches!(
            error.downcast_ref::<ProcessError>(),
            Some(ProcessError::Timeout { .. })
        ));
    }

    #[test]
    fn completion_terminates_detached_descendants() {
        let output = create_command("sleep 30 >/dev/null 2>&1 & printf $!")
            .exec_capture_output_blocking(&CaptureOptions::default())
            .unwrap();
        let pid = std::str::from_utf8(&output.stdout)
            .unwrap()
            .parse::<i32>()
            .unwrap();

        for _ in 0..20 {
            if unsafe { libc::kill(pid, 0) } == -1
                && std::io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH)
            {
                return;
            }

            std::thread::sleep(Duration::from_millis(10));
        }

        panic!("descendant process {pid} survived capture completion");
    }

    #[test]
    fn preserves_non_utf8_bytes() {
        let output = create_command(r"printf 'a\xffb'")
            .exec_capture_output_blocking(&CaptureOptions::default())
            .unwrap();

        assert_eq!(output.stdout.as_ref(), b"a\xffb");
    }

    #[test]
    fn captures_output_to_files() {
        let (stdout_path, stdout, stderr_path, stderr) = create_output_files();
        let output = create_command("printf 'out'; printf 'err' 1>&2")
            .exec_capture_output_to_files_blocking(&CaptureOptions::default(), stdout, stderr)
            .unwrap();

        assert_eq!(output.stdout_len, 3);
        assert_eq!(output.stderr_len, 3);
        assert_eq!(std::fs::read(&stdout_path).unwrap(), b"out");
        assert_eq!(std::fs::read(&stderr_path).unwrap(), b"err");

        std::fs::remove_file(stdout_path).unwrap();
        std::fs::remove_file(stderr_path).unwrap();
    }

    #[test]
    fn enforces_output_limit_when_capturing_to_files() {
        let (stdout_path, stdout, stderr_path, stderr) = create_output_files();
        let error = create_command("printf '123456789'")
            .exec_capture_output_to_files_blocking(
                &CaptureOptions {
                    output_limit: Some(8),
                    ..CaptureOptions::default()
                },
                stdout,
                stderr,
            )
            .unwrap_err();

        assert!(matches!(
            error.downcast_ref::<ProcessError>(),
            Some(ProcessError::OutputLimitExceeded { limit: 8, .. })
        ));

        std::fs::remove_file(stdout_path).unwrap();
        std::fs::remove_file(stderr_path).unwrap();
    }
}

mod exec_capture_continuous_output {
    use super::*;

    #[tokio::test]
    async fn pipes_input_and_captures_output() {
        let mut command = create_command("cat");
        command.set_continuous_pipe(true);
        command.input(["one\n", "two\n"]);

        let output = command.exec_capture_output().await.unwrap();

        assert!(output.success());
        assert_eq!(output.stdout.as_ref(), b"one\ntwo");
    }

    #[tokio::test]
    async fn survives_child_exiting_before_consuming_stdin() {
        // The child exits without reading stdin while we stream input far
        // larger than any pipe buffer, so the writer hits a broken pipe.
        // That must be benign: the child's exit status is the outcome
        // (moon previously died silently with exit code 141 here, when
        // SIGPIPE was reset to its default disposition).
        let mut command = create_command("exit 0");
        command.set_continuous_pipe(true);
        command.input(vec!["x".repeat(1024); 2048]);

        let output = command.exec_capture_output().await.unwrap();

        assert!(output.success());
    }
}

mod exec_stream_output {
    use super::*;

    #[tokio::test]
    async fn returns_empty_output() {
        let output = create_command("printf 'streamed'")
            .exec_stream_output()
            .await
            .unwrap();

        assert!(output.success());
        assert!(output.stdout.is_empty());
        assert!(output.stderr.is_empty());
    }

    #[tokio::test]
    async fn errors_on_nonzero_exit() {
        let error = create_command("exit 1")
            .exec_stream_output()
            .await
            .unwrap_err();

        assert!(matches!(
            error.downcast_ref::<ProcessError>().unwrap(),
            ProcessError::ExitNonZero { .. }
        ));
    }
}

mod exec_stream_and_capture_output {
    use super::*;

    #[tokio::test]
    async fn captures_lines_without_trailing_newline() {
        let output = create_command(r"printf 'a\nb\n'; printf 'err' 1>&2")
            .exec_stream_and_capture_output()
            .await
            .unwrap();

        assert!(output.success());
        assert_eq!(output.stdout.as_ref(), b"a\nb\n");
        assert_eq!(output.stderr.as_ref(), b"err");
    }
}

mod child_env {
    use super::*;

    #[tokio::test]
    async fn sets_env_vars() {
        let mut command = create_command(r#"printf "${MOON_TEST_SET_VAR:-missing}""#);
        command.env("MOON_TEST_SET_VAR", "value");

        let output = command.exec_capture_output().await.unwrap();

        assert_eq!(output.stdout.as_ref(), b"value");
    }

    #[tokio::test]
    async fn unsets_env_vars() {
        let mut command = create_command(r#"printf "${HOME:-unset}""#);
        command.env_remove("HOME");

        let output = command.exec_capture_output().await.unwrap();

        assert_eq!(output.stdout.as_ref(), b"unset");
    }

    #[tokio::test]
    async fn sets_cwd_and_pwd() {
        let dir = std::env::temp_dir().canonicalize().unwrap();

        let mut command = create_command(r#"printf "$PWD""#);
        command.cwd(&dir);

        let output = command.exec_capture_output().await.unwrap();

        assert_eq!(output.stdout, dir.as_os_str().as_encoded_bytes());
    }

    #[tokio::test]
    async fn prepends_lookup_paths() {
        let mut command = create_command(r#"printf "$PATH""#);
        command.prepend_paths(["/moon-test-fake-path"]);

        let output = command.exec_capture_output().await.unwrap();

        assert!(output.stdout.starts_with(b"/moon-test-fake-path:"));
    }
}

mod exec_stream_and_capture_output_bytes {
    use super::*;

    #[tokio::test]
    async fn captures_stdout_and_stderr() {
        let output = create_command("printf 'out'; printf 'err' 1>&2")
            .exec_stream_and_capture_output_bytes()
            .await
            .unwrap();

        assert!(output.success());
        assert_eq!(output.stdout.as_ref(), b"out");
        assert_eq!(output.stderr.as_ref(), b"err");
    }

    #[tokio::test]
    async fn preserves_non_utf8_bytes() {
        let output = create_command(r"printf 'a\xffb'")
            .exec_stream_and_capture_output_bytes()
            .await
            .unwrap();

        assert_eq!(output.stdout.as_ref(), b"a\xffb");
    }

    #[tokio::test]
    async fn collapses_carriage_return_redraws() {
        let output = create_command(r"printf '1/3\r2/3\r3/3 done\nnext\n'")
            .exec_stream_and_capture_output_bytes()
            .await
            .unwrap();

        assert_eq!(output.stdout.as_ref(), b"3/3 done\nnext\n");
    }

    #[tokio::test]
    async fn keeps_crlf_line_endings() {
        let output = create_command(r"printf 'one\r\ntwo\r\n'")
            .exec_stream_and_capture_output_bytes()
            .await
            .unwrap();

        assert_eq!(output.stdout.as_ref(), b"one\r\ntwo\r\n");
    }
}
