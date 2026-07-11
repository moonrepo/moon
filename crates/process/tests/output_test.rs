#[cfg(unix)]
fn create_output(exit: moon_process::ChildExit) -> moon_process::Output {
    moon_process::Output {
        exit,
        stdout: vec![],
        stderr: vec![],
    }
}

mod conversions {
    use moon_process::{output_to_string, output_to_trimmed_string};

    #[test]
    fn converts_bytes_to_string() {
        assert_eq!(output_to_string(b"hello"), "hello");
        assert_eq!(output_to_string(b""), "");
    }

    #[test]
    fn preserves_output_around_invalid_utf8() {
        assert_eq!(output_to_string(b"a\xffb"), "a\u{fffd}b");
    }

    #[test]
    fn trims_whitespace() {
        assert_eq!(output_to_trimmed_string(b"  hello\n"), "hello");
    }
}

#[cfg(unix)]
mod statuses {
    use super::*;
    use moon_process::ChildExit;
    use std::os::unix::process::ExitStatusExt;
    use std::process::ExitStatus;

    #[test]
    fn completed_zero_is_success() {
        let output = create_output(ChildExit::Completed(ExitStatus::from_raw(0)));

        assert!(output.success());
        assert_eq!(output.code(), Some(0));
    }

    #[test]
    fn completed_nonzero_is_failure() {
        // Wait statuses encode the exit code in the upper byte
        let output = create_output(ChildExit::Completed(ExitStatus::from_raw(2 << 8)));

        assert!(!output.success());
        assert_eq!(output.code(), Some(2));
    }

    #[test]
    fn signal_exits_are_failures_without_codes() {
        for exit in [
            ChildExit::Interrupted,
            ChildExit::Killed,
            ChildExit::Terminated,
        ] {
            let output = create_output(exit);

            assert!(!output.success());
            assert_eq!(output.code(), None);
            assert_eq!(output.status(), None);
        }
    }
}

#[cfg(unix)]
mod errors {
    use super::*;
    use moon_process::{ChildExit, ProcessError};
    use std::os::unix::process::ExitStatusExt;
    use std::process::ExitStatus;

    fn create_failed_output() -> moon_process::Output {
        create_output(ChildExit::Completed(ExitStatus::from_raw(1 << 8)))
    }

    #[test]
    fn formats_exit_code_without_message() {
        let ProcessError::ExitNonZero { bin, status, code } =
            create_failed_output().to_error("git", false)
        else {
            panic!("expected ExitNonZero");
        };

        assert_eq!(bin, "git");
        assert_eq!(status, "exit code 1");
        assert_eq!(code, Some(1));
    }

    #[test]
    fn formats_signal_statuses() {
        for (exit, label) in [
            (ChildExit::Interrupted, "interrupted"),
            (ChildExit::Killed, "killed"),
            (ChildExit::Terminated, "terminated"),
        ] {
            let ProcessError::ExitNonZero { status, code, .. } =
                create_output(exit).to_error("git", false)
            else {
                panic!("expected ExitNonZero");
            };

            assert_eq!(status, label);
            // Signals have no exit code, so we can't propagate one
            assert_eq!(code, None);
        }
    }

    #[test]
    fn prefers_stderr_for_message() {
        let mut output = create_failed_output();
        output.stdout = b"from stdout".to_vec();
        output.stderr = b"from stderr\n".to_vec();

        let ProcessError::ExitNonZeroWithOutput {
            output: message, ..
        } = output.to_error("git", true)
        else {
            panic!("expected ExitNonZeroWithOutput");
        };

        assert_eq!(message, "\n\nfrom stderr");
    }

    #[test]
    fn falls_back_to_stdout_for_message() {
        let mut output = create_failed_output();
        output.stdout = b"from stdout".to_vec();

        let ProcessError::ExitNonZeroWithOutput {
            output: message, ..
        } = output.to_error("git", true)
        else {
            panic!("expected ExitNonZeroWithOutput");
        };

        assert_eq!(message, "\n\nfrom stdout");
    }

    #[test]
    fn empty_output_has_empty_message() {
        let ProcessError::ExitNonZeroWithOutput {
            output: message, ..
        } = create_failed_output().to_error("git", true)
        else {
            panic!("expected ExitNonZeroWithOutput");
        };

        assert_eq!(message, "");
    }
}
