use miette::Diagnostic;
use moon_app::extract_task_exit_code;
use moon_process::ProcessError;
use thiserror::Error;

// Mirrors how `TaskRunnerError::RunFailed` wraps `Box<ProcessError>`, which
// surfaces the process error as a `Box<ProcessError>` in the error chain.
#[derive(Debug, Diagnostic, Error)]
#[error("task failed to run")]
struct WrappedError {
    #[source]
    error: Box<ProcessError>,
}

fn exit_nonzero(code: Option<i32>) -> ProcessError {
    ProcessError::ExitNonZero {
        bin: "bash".into(),
        status: "exit code".into(),
        code,
    }
}

fn wrapped(code: Option<i32>) -> miette::Report {
    WrappedError {
        error: Box::new(exit_nonzero(code)),
    }
    .into()
}

mod extract_task_exit_code {
    use super::*;

    #[test]
    fn extracts_from_boxed_process_error() {
        assert_eq!(extract_task_exit_code(&wrapped(Some(80))), Some(80));
    }

    #[test]
    fn extracts_from_bare_process_error() {
        let report: miette::Report = exit_nonzero(Some(2)).into();

        assert_eq!(extract_task_exit_code(&report), Some(2));
    }

    #[test]
    fn propagates_codes_above_127() {
        assert_eq!(extract_task_exit_code(&wrapped(Some(200))), Some(200));
    }

    #[test]
    fn none_for_signal_without_code() {
        assert_eq!(extract_task_exit_code(&wrapped(None)), None);
    }

    #[test]
    fn none_for_injected_abort_code() {
        // Aborts before a process spawns are recorded with an exit code of -1
        assert_eq!(extract_task_exit_code(&wrapped(Some(-1))), None);
    }

    #[test]
    fn none_for_zero_code() {
        assert_eq!(extract_task_exit_code(&wrapped(Some(0))), None);
    }

    #[test]
    fn none_for_out_of_u8_range() {
        // e.g. large Windows exit codes that can't map to a u8
        assert_eq!(extract_task_exit_code(&wrapped(Some(300))), None);
    }

    #[test]
    fn none_for_non_process_error() {
        let report = miette::miette!("something else went wrong");

        assert_eq!(extract_task_exit_code(&report), None);
    }
}
