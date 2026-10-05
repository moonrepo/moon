use moon_task::TaskExpectedFailure;

mod task_expected_failure {
    use super::*;

    #[test]
    fn only_a_normal_failure_is_expected() {
        for (code, expected) in [
            (0, TaskExpectedFailure::Passed),
            (1, TaskExpectedFailure::Failed),
            (2, TaskExpectedFailure::Failed),
            (125, TaskExpectedFailure::Failed),
            (126, TaskExpectedFailure::NotExecutable),
            (127, TaskExpectedFailure::NotFound),
            (128, TaskExpectedFailure::Failed),
            (129, TaskExpectedFailure::Signaled(1)),
            (130, TaskExpectedFailure::Signaled(2)),
            (192, TaskExpectedFailure::Signaled(64)),
            (193, TaskExpectedFailure::Failed),
            (255, TaskExpectedFailure::Failed),
            (256, TaskExpectedFailure::Abnormal(256)),
            (-1, TaskExpectedFailure::Abnormal(-1)),
            // Windows access violation (0xC0000005)
            (-1073741819, TaskExpectedFailure::Abnormal(-1073741819)),
        ] {
            let outcome = TaskExpectedFailure::from_exit_code(code);

            assert_eq!(outcome, expected, "exit code {code}");
            assert_eq!(
                outcome.is_expected(),
                expected == TaskExpectedFailure::Failed,
                "exit code {code}"
            );
        }
    }

    #[test]
    fn describes_the_outcome() {
        assert_eq!(TaskExpectedFailure::Passed.to_string(), "passed");
        assert_eq!(
            TaskExpectedFailure::NotFound.to_string(),
            "exited with code 127 (command not found)"
        );
        assert_eq!(
            TaskExpectedFailure::from_exit_code(137).to_string(),
            "exited with code 137 (killed by signal 9)"
        );
    }
}
