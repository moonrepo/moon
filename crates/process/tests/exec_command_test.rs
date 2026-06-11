#![cfg(unix)]

use moon_console::Console;
use moon_process::Command;
use std::sync::Arc;

fn create_command(script: &str) -> Command {
    let mut command = Command::new("bash");
    command.args(["-c", script]);
    command.no_shell();
    command.set_console(Arc::new(Console::new_testing()));
    command
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
