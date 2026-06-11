#![cfg(unix)]

use moon_console::Console;
use moon_process::{ChildExit, Command, ProcessError};
use std::sync::Arc;

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
        assert_eq!(output.stdout, b"out");
        assert_eq!(output.stderr, b"err");
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

        assert_eq!(output.stdout, b"hello world");
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

mod exec_capture_continuous_output {
    use super::*;

    #[tokio::test]
    async fn pipes_input_and_captures_output() {
        let mut command = create_command("cat");
        command.set_continuous_pipe(true);
        command.input(["one\n", "two\n"]);

        let output = command.exec_capture_output().await.unwrap();

        assert!(output.success());
        assert_eq!(output.stdout, b"one\ntwo");
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
        assert_eq!(output.stdout, b"a\nb");
        assert_eq!(output.stderr, b"err");
    }
}

mod child_env {
    use super::*;

    #[tokio::test]
    async fn sets_env_vars() {
        let mut command = create_command(r#"printf "${MOON_TEST_SET_VAR:-missing}""#);
        command.env("MOON_TEST_SET_VAR", "value");

        let output = command.exec_capture_output().await.unwrap();

        assert_eq!(output.stdout, b"value");
    }

    #[tokio::test]
    async fn unsets_env_vars() {
        let mut command = create_command(r#"printf "${HOME:-unset}""#);
        command.env_remove("HOME");

        let output = command.exec_capture_output().await.unwrap();

        assert_eq!(output.stdout, b"unset");
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
        assert_eq!(output.stdout, b"out");
        assert_eq!(output.stderr, b"err");
    }

    #[tokio::test]
    async fn preserves_non_utf8_bytes() {
        let output = create_command(r"printf 'a\xffb'")
            .exec_stream_and_capture_output_bytes()
            .await
            .unwrap();

        assert_eq!(output.stdout, b"a\xffb");
    }

    #[tokio::test]
    async fn collapses_carriage_return_redraws() {
        let output = create_command(r"printf '1/3\r2/3\r3/3 done\nnext\n'")
            .exec_stream_and_capture_output_bytes()
            .await
            .unwrap();

        assert_eq!(output.stdout, b"3/3 done\nnext\n");
    }

    #[tokio::test]
    async fn keeps_crlf_line_endings() {
        let output = create_command(r"printf 'one\r\ntwo\r\n'")
            .exec_stream_and_capture_output_bytes()
            .await
            .unwrap();

        assert_eq!(output.stdout, b"one\r\ntwo\r\n");
    }
}
