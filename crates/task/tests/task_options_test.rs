use moon_config::test_utils::stub_file_input;
use moon_config::{
    Input, MergeStrategy, TaskOperatingSystem, TaskOptionAffectedFilesPattern, TaskOptionCache,
    TaskOptionEnvOverride, TaskOptionRunInCI, TaskOutputStyle, TaskPriority, TaskUnixShell,
    TaskWindowsShell,
};
use moon_task::{TaskExpectedFailure, TaskOptionAffectedFiles, TaskOptions};

mod task_options {
    use super::*;

    #[test]
    fn doesnt_serialize_defaults() {
        assert_eq!(
            serde_json::to_string(&TaskOptions::default()).unwrap(),
            "{}"
        );
        assert_eq!(
            serde_json::to_string(&TaskOptionAffectedFiles::default()).unwrap(),
            "{}"
        );
    }

    #[test]
    fn deserializes_missing_fields_as_defaults() {
        assert_eq!(
            serde_json::from_str::<TaskOptions>("{}").unwrap(),
            TaskOptions::default()
        );
        assert_eq!(
            serde_json::from_str::<TaskOptionAffectedFiles>("{}").unwrap(),
            TaskOptionAffectedFiles::default()
        );
    }

    #[test]
    fn serializes_and_round_trips_non_defaults() {
        // Every field is set without `..Default::default()`, so that
        // new fields must be added here
        let options = TaskOptions {
            affected_files: Some(TaskOptionAffectedFiles {
                filter: vec!["*.js".into()],
                ignore_project_boundary: true,
                pass: TaskOptionAffectedFilesPattern::Args,
                pass_inputs_when_no_match: true,
                pass_dot_when_no_results: true,
            }),
            allow_failure: true,
            cache: TaskOptionCache::Enabled(false),
            cache_key: Some("key".into()),
            cache_lifetime: Some("1 day".into()),
            env_files: Some(vec![Input::File(stub_file_input(".env"))]),
            env_override: TaskOptionEnvOverride::Enabled(true),
            expect_failure: true,
            infer_inputs: true,
            internal: true,
            interactive: true,
            merge_args: MergeStrategy::Replace,
            merge_checks: MergeStrategy::Replace,
            merge_deps: MergeStrategy::Replace,
            merge_env: MergeStrategy::Replace,
            merge_inputs: MergeStrategy::Replace,
            merge_outputs: MergeStrategy::Replace,
            merge_tags: MergeStrategy::Replace,
            merge_toolchains: MergeStrategy::Replace,
            mutex: Some("lock".into()),
            os: Some(vec![TaskOperatingSystem::Linux]),
            output_style: Some(TaskOutputStyle::Stream),
            persistent: true,
            priority: TaskPriority::High,
            retry_count: 3,
            run_deps_in_parallel: false,
            run_in_ci: TaskOptionRunInCI::Enabled(false),
            run_in_sync_phase: true,
            run_from_workspace_root: true,
            // Disabled by default, so a missing value must stay missing
            shell: None,
            timeout: Some(60),
            unix_shell: TaskUnixShell::Zsh,
            windows_shell: TaskWindowsShell::Bash,
        };

        let json = serde_json::to_value(&options).unwrap();

        assert_eq!(json.as_object().unwrap().len(), 33);
        assert_eq!(json["affectedFiles"].as_object().unwrap().len(), 5);
        assert_eq!(
            serde_json::from_value::<TaskOptions>(json).unwrap(),
            options
        );
    }
}

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
