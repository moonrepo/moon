mod utils;

use moon_app::queries::tasks::QueryTasksResult;
use moon_common::is_ci;
use starbase_utils::json::serde_json;
use utils::{change_files, create_query_sandbox, create_tasks_sandbox};

fn extract_targets(result: &QueryTasksResult) -> Vec<String> {
    result.tasks.iter().fold(vec![], |mut acc, (_, tasks)| {
        acc.extend(tasks.values().map(|t| t.target.to_string()));
        acc
    })
}

mod query_tasks {
    use super::*;

    #[test]
    fn returns_all_by_default() {
        let sandbox = create_query_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("query").arg("tasks");
        });

        let json: QueryTasksResult = serde_json::from_str(assert.stdout().trim()).unwrap();
        let tasks = json
            .tasks
            .get("tasks")
            .unwrap()
            .keys()
            .map(|k| k.to_owned())
            .collect::<Vec<_>>();
        let mut projects = json.tasks.into_keys().collect::<Vec<_>>();

        projects.sort();

        assert_eq!(tasks, ["lint", "test"]);
        assert_eq!(
            projects,
            ["advanced", "basic", "metadata", "tasks", "toolchains"]
        );
    }

    #[test]
    fn can_filter_by_affected() {
        let sandbox = create_query_sandbox();

        change_files(&sandbox, ["metadata/file.txt"]);

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("query").arg("tasks").arg("--affected");
        });

        let json: QueryTasksResult = serde_json::from_str(assert.stdout().trim()).unwrap();
        let targets = extract_targets(&json);

        assert_eq!(targets, ["metadata:build", "metadata:test"]);
        assert!(json.options.affected.is_some());
    }

    #[test]
    fn can_filter_by_affected_via_stdin() {
        let sandbox = create_query_sandbox();

        change_files(&sandbox, ["metadata/file.txt"]);

        let query = sandbox.run_bin(|cmd| {
            cmd.arg("query").arg("changed-files");

            if !is_ci() {
                cmd.arg("--local");
            }
        });

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("query")
                .arg("tasks")
                .arg("--affected")
                .write_stdin(query.stdout());
        });

        let json: QueryTasksResult = serde_json::from_str(assert.stdout().trim()).unwrap();
        let targets = extract_targets(&json);

        assert_eq!(targets, ["metadata:build", "metadata:test"]);
        assert!(json.options.affected.is_some());
    }

    #[test]
    fn can_filter_by_id() {
        let sandbox = create_query_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("query").arg("tasks").args(["--id", "te(st|m)"]);
        });

        let json: QueryTasksResult = serde_json::from_str(assert.stdout().trim()).unwrap();
        let targets = extract_targets(&json);

        assert_eq!(
            targets,
            ["metadata:test", "tasks:test", "toolchains:system"]
        );
        assert_eq!(json.options.id.unwrap(), "te(st|m)".to_string());
    }

    #[test]
    fn can_filter_by_command() {
        let sandbox = create_query_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("query").arg("tasks").args(["--command", "noop"]);
        });

        let json: QueryTasksResult = serde_json::from_str(assert.stdout().trim()).unwrap();
        let targets = extract_targets(&json);

        assert_eq!(
            targets,
            ["metadata:build", "metadata:test", "toolchains:system"]
        );
        assert_eq!(json.options.command.unwrap(), "noop".to_string());
    }

    #[test]
    fn can_filter_by_toolchain() {
        // Projects sandbox doesn't have toolchains enabled
        let sandbox = create_tasks_sandbox();
        sandbox.enable_git();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("query")
                .arg("tasks")
                .args(["--toolchain", "(system|type)"]);
        });

        let json: QueryTasksResult = serde_json::from_str(assert.stdout().trim()).unwrap();
        let targets = extract_targets(&json);

        assert_eq!(targets, ["basic:build", "basic:lint", "basic:test"]);
        assert_eq!(json.options.toolchain.unwrap(), "(system|type)".to_string());
    }

    #[test]
    fn can_filter_by_project() {
        let sandbox = create_query_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("query").arg("tasks").args(["--project", "a(d|i)"]);
        });

        let json: QueryTasksResult = serde_json::from_str(assert.stdout().trim()).unwrap();
        let targets = extract_targets(&json);

        assert_eq!(
            targets,
            [
                "advanced:build",
                "metadata:build",
                "metadata:test",
                "toolchains:node",
                "toolchains:system"
            ]
        );
        assert_eq!(json.options.project.unwrap(), "a(d|i)".to_string());
    }

    #[test]
    fn can_filter_by_type() {
        let sandbox = create_query_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("query").arg("tasks").args(["--type", "build"]);
        });

        let json: QueryTasksResult = serde_json::from_str(assert.stdout().trim()).unwrap();
        let targets = extract_targets(&json);

        assert_eq!(targets, ["advanced:build", "tasks:lint"]);
        assert_eq!(json.options.type_of.unwrap(), "build".to_string());
    }

    mod mql {
        use super::*;

        #[test]
        fn can_filter_with_query() {
            let sandbox = create_query_sandbox();

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("query").arg("tasks").arg("task=test");
            });

            let json: QueryTasksResult = serde_json::from_str(assert.stdout().trim()).unwrap();
            let targets = extract_targets(&json);

            assert_eq!(targets, ["metadata:test", "tasks:test"]);
            assert_eq!(json.options.query.unwrap(), "task=test".to_string());
        }

        #[test]
        fn can_filter_by_affected_with_query() {
            let sandbox = create_query_sandbox();

            change_files(&sandbox, ["metadata/file.txt"]);

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("query")
                    .arg("tasks")
                    .arg("task=test")
                    .arg("--affected");
            });

            let json: QueryTasksResult = serde_json::from_str(assert.stdout().trim()).unwrap();
            let targets = extract_targets(&json);

            assert_eq!(targets, ["metadata:test"]);
            assert_eq!(json.options.query.unwrap(), "task=test".to_string());
        }
    }
}
