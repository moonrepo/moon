use moon_cli::enums::TouchedStatus;
use moon_cli::queries::projects::{QueryProjectsResult, QueryTasksResult};
use moon_cli::queries::touched_files::QueryTouchedFilesResult;
use moon_test_utils::{
    assert_snapshot, create_sandbox_with_config, get_assert_stdout_output,
    get_cases_fixture_configs, get_projects_fixture_configs, predicates::prelude::*, Sandbox,
};
use moon_utils::{is_ci, string_vec};

fn change_branch(sandbox: &Sandbox) {
    sandbox.run_git(|cmd| {
        cmd.args(["checkout", "-b", "branch"]);
    });
}

fn touch_file(sandbox: &Sandbox) {
    sandbox.create_file("advanced/file", "contents");
    sandbox.create_file("no-config/file", "contents");

    // CI uses `git diff` while local uses `git status`
    if is_ci() {
        change_branch(sandbox);

        sandbox.run_git(|cmd| {
            cmd.args(["add", "advanced/file", "no-config/file"]);
        });

        sandbox.run_git(|cmd| {
            cmd.args(["commit", "-m", "Touch"]);
        });
    }
}

mod hash {
    use super::*;
    use std::fs;

    #[test]
    fn errors_if_hash_doesnt_exist() {
        let sandbox = create_sandbox_with_config("base", None, None, None);

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("query").arg("hash").arg("a");
        });

        let output = assert.output();

        assert!(predicate::str::contains("Unable to find a hash manifest for a!").eval(&output));
    }

    #[test]
    fn prints_the_manifest() {
        let sandbox = create_sandbox_with_config("base", None, None, None);

        fs::create_dir_all(sandbox.path().join(".moon/cache/hashes")).unwrap();

        fs::write(
            sandbox.path().join(".moon/cache/hashes/a.json"),
            r#"{
    "command": "base",
    "args": [
        "a",
        "b",
        "c"
    ]
}"#,
        )
        .unwrap();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("query").arg("hash").arg("a");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn prints_the_manifest_in_json() {
        let sandbox = create_sandbox_with_config("base", None, None, None);

        fs::create_dir_all(sandbox.path().join(".moon/cache/hashes")).unwrap();

        fs::write(
            sandbox.path().join(".moon/cache/hashes/a.json"),
            r#"{
    "command": "base",
    "args": [
        "a",
        "b",
        "c"
    ]
}"#,
        )
        .unwrap();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("query").arg("hash").arg("a").arg("--json");
        });

        assert_snapshot!(assert.output());
    }
}

mod hash_diff {
    use super::*;
    use std::fs;

    #[test]
    fn errors_if_left_doesnt_exist() {
        let sandbox = create_sandbox_with_config("base", None, None, None);

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("query").arg("hash-diff").arg("a").arg("b");
        });

        let output = assert.output();

        assert!(predicate::str::contains("Unable to find a hash manifest for a!").eval(&output));
    }

    #[test]
    fn errors_if_right_doesnt_exist() {
        let sandbox = create_sandbox_with_config("base", None, None, None);

        fs::create_dir_all(sandbox.path().join(".moon/cache/hashes")).unwrap();

        fs::write(
            sandbox.path().join(".moon/cache/hashes/a.json"),
            r#"{
    "command": "test",
    "args": [
        "a",
        "b",
        "c"
    ]
}"#,
        )
        .unwrap();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("query").arg("hash-diff").arg("a").arg("b");
        });

        let output = assert.output();

        assert!(predicate::str::contains("Unable to find a hash manifest for b!").eval(&output));
    }

    #[test]
    fn prints_a_diff() {
        let sandbox = create_sandbox_with_config("base", None, None, None);

        fs::create_dir_all(sandbox.path().join(".moon/cache/hashes")).unwrap();

        fs::write(
            sandbox.path().join(".moon/cache/hashes/a.json"),
            r#"{
    "command": "base",
    "args": [
        "a",
        "b",
        "c"
    ]
}"#,
        )
        .unwrap();

        fs::write(
            sandbox.path().join(".moon/cache/hashes/b.json"),
            r#"{
    "command": "other",
    "args": [
        "a",
        "123",
        "c"
    ]
}"#,
        )
        .unwrap();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("query").arg("hash-diff").arg("a").arg("b");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn prints_a_diff_in_json() {
        let sandbox = create_sandbox_with_config("base", None, None, None);

        fs::create_dir_all(sandbox.path().join(".moon/cache/hashes")).unwrap();

        fs::write(
            sandbox.path().join(".moon/cache/hashes/a.json"),
            r#"{
    "command": "base",
    "args": [
        "a",
        "b",
        "c"
    ]
}"#,
        )
        .unwrap();

        fs::write(
            sandbox.path().join(".moon/cache/hashes/b.json"),
            r#"{
    "command": "other",
    "args": [
        "a",
        "123",
        "c"
    ]
}"#,
        )
        .unwrap();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("query")
                .arg("hash-diff")
                .arg("a")
                .arg("b")
                .arg("--json");
        });

        assert_snapshot!(assert.output());
    }
}

mod projects {
    use super::*;

    #[test]
    fn returns_all_by_default() {
        let (workspace_config, toolchain_config, tasks_config) = get_projects_fixture_configs();

        let sandbox = create_sandbox_with_config(
            "projects",
            Some(workspace_config),
            Some(toolchain_config),
            Some(tasks_config),
        );

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("query").arg("projects");
        });

        assert_snapshot!(assert.output_standardized());
    }

    #[test]
    fn returns_all_by_default_json() {
        let (workspace_config, toolchain_config, tasks_config) = get_projects_fixture_configs();

        let sandbox = create_sandbox_with_config(
            "projects",
            Some(workspace_config),
            Some(toolchain_config),
            Some(tasks_config),
        );

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("query").arg("projects").arg("--json");
        });

        let json: QueryProjectsResult = serde_json::from_str(&assert.output()).unwrap();
        let mut ids: Vec<String> = json.projects.iter().map(|p| p.id.to_string()).collect();

        ids.sort();

        assert_eq!(
            ids,
            string_vec![
                "advanced",
                "bar",
                "basic",
                "baz",
                "emptyConfig",
                "foo",
                "metadata",
                "noConfig",
                "platforms",
                "tasks",
            ]
        );
    }

    #[test]
    fn can_filter_by_affected() {
        let (workspace_config, toolchain_config, tasks_config) = get_projects_fixture_configs();

        let sandbox = create_sandbox_with_config(
            "projects",
            Some(workspace_config),
            Some(toolchain_config),
            Some(tasks_config),
        );
        sandbox.enable_git();

        touch_file(&sandbox);

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("query")
                .arg("projects")
                .arg("--json")
                .arg("--affected");
        });

        let json: QueryProjectsResult = serde_json::from_str(&assert.output()).unwrap();
        let ids: Vec<String> = json.projects.iter().map(|p| p.id.to_string()).collect();

        assert_eq!(ids, string_vec!["advanced", "noConfig"]);
        assert!(json.options.affected);
    }

    #[test]
    fn can_filter_by_affected_via_stdin() {
        let (workspace_config, toolchain_config, tasks_config) = get_projects_fixture_configs();

        let sandbox = create_sandbox_with_config(
            "projects",
            Some(workspace_config),
            Some(toolchain_config),
            Some(tasks_config),
        );
        sandbox.enable_git();

        touch_file(&sandbox);

        let query = sandbox.run_moon(|cmd| {
            cmd.arg("query").arg("touched-files");

            if !is_ci() {
                cmd.arg("--local");
            }
        });

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("query")
                .arg("projects")
                .arg("--affected")
                .write_stdin(get_assert_stdout_output(&query.inner));
        });

        assert_eq!(
            assert.output(),
            "advanced | advanced | frontend | application | typescript\nnoConfig | no-config | unknown | unknown | unknown\n\n"
        );
    }

    #[test]
    fn can_filter_by_affected_via_stdin_json() {
        let (workspace_config, toolchain_config, tasks_config) = get_projects_fixture_configs();

        let sandbox = create_sandbox_with_config(
            "projects",
            Some(workspace_config),
            Some(toolchain_config),
            Some(tasks_config),
        );
        sandbox.enable_git();

        touch_file(&sandbox);

        let query = sandbox.run_moon(|cmd| {
            cmd.arg("query").arg("touched-files").arg("--json");

            if !is_ci() {
                cmd.arg("--local");
            }
        });

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("query")
                .arg("projects")
                .arg("--json")
                .arg("--affected")
                .write_stdin(get_assert_stdout_output(&query.inner));
        });

        let json: QueryProjectsResult = serde_json::from_str(&assert.output()).unwrap();
        let ids: Vec<String> = json.projects.iter().map(|p| p.id.to_string()).collect();

        assert_eq!(ids, string_vec!["advanced", "noConfig"]);
        assert!(json.options.affected);
    }

    #[test]
    fn can_filter_by_id() {
        let (workspace_config, toolchain_config, tasks_config) = get_projects_fixture_configs();

        let sandbox = create_sandbox_with_config(
            "projects",
            Some(workspace_config),
            Some(toolchain_config),
            Some(tasks_config),
        );

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("query")
                .arg("projects")
                .arg("--json")
                .args(["--id", "ba(r|z)"]);
        });

        let json: QueryProjectsResult = serde_json::from_str(&assert.output()).unwrap();
        let ids: Vec<String> = json.projects.iter().map(|p| p.id.to_string()).collect();

        assert_eq!(ids, string_vec!["bar", "baz"]);
        assert_eq!(json.options.id.unwrap(), "ba(r|z)".to_string());
    }

    #[test]
    fn can_filter_by_source() {
        let (workspace_config, toolchain_config, tasks_config) = get_projects_fixture_configs();

        let sandbox = create_sandbox_with_config(
            "projects",
            Some(workspace_config),
            Some(toolchain_config),
            Some(tasks_config),
        );

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("query")
                .arg("projects")
                .arg("--json")
                .args(["--source", "config$"]);
        });

        let json: QueryProjectsResult = serde_json::from_str(&assert.output()).unwrap();
        let ids: Vec<String> = json.projects.iter().map(|p| p.id.to_string()).collect();

        assert_eq!(ids, string_vec!["emptyConfig", "noConfig"]);
        assert_eq!(json.options.source.unwrap(), "config$".to_string());
    }

    #[test]
    fn can_filter_by_tags() {
        let (workspace_config, toolchain_config, tasks_config) = get_projects_fixture_configs();

        let sandbox = create_sandbox_with_config(
            "projects",
            Some(workspace_config),
            Some(toolchain_config),
            Some(tasks_config),
        );

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("query")
                .arg("projects")
                .arg("--json")
                .args(["--tags", "react"]);
        });

        let json: QueryProjectsResult = serde_json::from_str(&assert.output()).unwrap();
        let ids: Vec<String> = json.projects.iter().map(|p| p.id.to_string()).collect();

        assert_eq!(ids, string_vec!["advanced", "foo"]);
        assert_eq!(json.options.tags.unwrap(), "react".to_string());

        // Multiple
        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("query")
                .arg("projects")
                .arg("--json")
                .args(["--tags", "react|vue"]);
        });

        let json: QueryProjectsResult = serde_json::from_str(&assert.output()).unwrap();
        let ids: Vec<String> = json.projects.iter().map(|p| p.id.to_string()).collect();

        assert_eq!(ids, string_vec!["advanced", "basic", "foo"]);
        assert_eq!(json.options.tags.unwrap(), "react|vue".to_string());
    }

    #[test]
    fn can_filter_by_tasks() {
        let (workspace_config, toolchain_config, tasks_config) = get_projects_fixture_configs();

        let sandbox = create_sandbox_with_config(
            "projects",
            Some(workspace_config),
            Some(toolchain_config),
            Some(tasks_config),
        );

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("query")
                .arg("projects")
                .arg("--json")
                .args(["--tasks", "lint"]);
        });

        let json: QueryProjectsResult = serde_json::from_str(&assert.output()).unwrap();
        let ids: Vec<String> = json.projects.iter().map(|p| p.id.to_string()).collect();

        assert_eq!(ids, string_vec!["tasks"]);
        assert_eq!(json.options.tasks.unwrap(), "lint".to_string());
    }

    #[test]
    fn can_filter_by_language() {
        let (workspace_config, toolchain_config, tasks_config) = get_projects_fixture_configs();

        let sandbox = create_sandbox_with_config(
            "projects",
            Some(workspace_config),
            Some(toolchain_config),
            Some(tasks_config),
        );

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("query")
                .arg("projects")
                .arg("--json")
                .args(["--language", "java|bash"]);
        });

        let json: QueryProjectsResult = serde_json::from_str(&assert.output()).unwrap();
        let ids: Vec<String> = json.projects.iter().map(|p| p.id.to_string()).collect();

        assert_eq!(ids, string_vec!["basic", "foo"]);
        assert_eq!(json.options.language.unwrap(), "java|bash".to_string());
    }

    #[test]
    fn can_filter_by_type() {
        let (workspace_config, toolchain_config, tasks_config) = get_projects_fixture_configs();

        let sandbox = create_sandbox_with_config(
            "projects",
            Some(workspace_config),
            Some(toolchain_config),
            Some(tasks_config),
        );

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("query")
                .arg("projects")
                .arg("--json")
                .args(["--type", "app"]);
        });

        let json: QueryProjectsResult = serde_json::from_str(&assert.output()).unwrap();
        let ids: Vec<String> = json.projects.iter().map(|p| p.id.to_string()).collect();

        assert_eq!(ids, string_vec!["advanced", "foo"]);
        assert_eq!(json.options.type_of.unwrap(), "app".to_string());
    }

    #[test]
    fn can_filter_by_stack() {
        let (workspace_config, toolchain_config, tasks_config) = get_projects_fixture_configs();

        let sandbox = create_sandbox_with_config(
            "projects",
            Some(workspace_config),
            Some(toolchain_config),
            Some(tasks_config),
        );

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("query")
                .arg("projects")
                .arg("--json")
                .args(["--stack", "frontend"]);
        });

        let json: QueryProjectsResult = serde_json::from_str(&assert.output()).unwrap();
        let ids: Vec<String> = json.projects.iter().map(|p| p.id.to_string()).collect();

        assert_eq!(ids, string_vec!["advanced"]);
        assert_eq!(json.options.stack.unwrap(), "frontend".to_string());
    }

    mod mql {
        use super::*;

        #[test]
        fn can_filter_with_query() {
            let (workspace_config, toolchain_config, tasks_config) = get_projects_fixture_configs();

            let sandbox = create_sandbox_with_config(
                "projects",
                Some(workspace_config),
                Some(toolchain_config),
                Some(tasks_config),
            );

            let assert = sandbox.run_moon(|cmd| {
                cmd.arg("query")
                    .arg("projects")
                    .arg("project~ba{r,z}")
                    .arg("--json");
            });

            let json: QueryProjectsResult = serde_json::from_str(&assert.output()).unwrap();
            let ids: Vec<String> = json.projects.iter().map(|p| p.id.to_string()).collect();

            assert_eq!(ids, string_vec!["bar", "baz"]);
            assert_eq!(json.options.query.unwrap(), "project~ba{r,z}".to_string());
        }

        #[test]
        fn can_filter_by_affected_with_query() {
            let (workspace_config, toolchain_config, tasks_config) = get_projects_fixture_configs();

            let sandbox = create_sandbox_with_config(
                "projects",
                Some(workspace_config),
                Some(toolchain_config),
                Some(tasks_config),
            );
            sandbox.enable_git();

            touch_file(&sandbox);

            let assert = sandbox.run_moon(|cmd| {
                cmd.arg("query")
                    .arg("projects")
                    .arg("project~*Config")
                    .arg("--json")
                    .arg("--affected");
            });

            let json: QueryProjectsResult = serde_json::from_str(&assert.output()).unwrap();
            let ids: Vec<String> = json.projects.iter().map(|p| p.id.to_string()).collect();

            assert_eq!(ids, string_vec!["noConfig"]);
            assert!(json.options.affected);
        }
    }
}

mod tasks {
    use super::*;

    #[test]
    fn returns_all_by_default() {
        let (workspace_config, toolchain_config, tasks_config) = get_projects_fixture_configs();

        let sandbox = create_sandbox_with_config(
            "projects",
            Some(workspace_config),
            Some(toolchain_config),
            Some(tasks_config),
        );

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("query").arg("tasks");
        });

        assert_snapshot!(assert.output_standardized());
    }

    #[test]
    fn returns_all_by_default_json() {
        let (workspace_config, toolchain_config, tasks_config) = get_projects_fixture_configs();

        let sandbox = create_sandbox_with_config(
            "projects",
            Some(workspace_config),
            Some(toolchain_config),
            Some(tasks_config),
        );

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("query").arg("tasks").arg("--json");
        });

        let json: QueryTasksResult = serde_json::from_str(&assert.output()).unwrap();
        let tasks = json
            .tasks
            .get("tasks")
            .unwrap()
            .keys()
            .map(|k| k.to_owned())
            .collect::<Vec<_>>();
        let mut projects = json.tasks.into_keys().collect::<Vec<_>>();

        projects.sort();

        assert_eq!(tasks, string_vec!["internal", "lint", "test"]);
        assert_eq!(projects, string_vec!["metadata", "platforms", "tasks"]);
    }
}

mod touched_files {
    use super::*;

    #[test]
    fn can_change_options() {
        let (workspace_config, toolchain_config, tasks_config) = get_cases_fixture_configs();

        let sandbox = create_sandbox_with_config(
            "cases",
            Some(workspace_config),
            Some(toolchain_config),
            Some(tasks_config),
        );
        sandbox.enable_git();

        change_branch(&sandbox);

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("query").arg("touched-files").args([
                "--base", "master", "--head", "branch", "--status", "deleted", "--json",
            ]);
        });

        let json: QueryTouchedFilesResult = serde_json::from_str(&assert.output()).unwrap();

        assert_eq!(json.options.base.unwrap(), "master".to_string());
        assert_eq!(json.options.head.unwrap(), "branch".to_string());
        assert_eq!(json.options.status, vec![TouchedStatus::Deleted]);
        assert!(!json.options.local);
    }

    #[test]
    fn can_supply_multi_status() {
        let (workspace_config, toolchain_config, tasks_config) = get_cases_fixture_configs();

        let sandbox = create_sandbox_with_config(
            "cases",
            Some(workspace_config),
            Some(toolchain_config),
            Some(tasks_config),
        );
        sandbox.enable_git();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("query").arg("touched-files").args([
                "--status", "deleted", "--status", "added", "--status", "modified", "--json",
            ]);
        });

        let json: QueryTouchedFilesResult = serde_json::from_str(&assert.output()).unwrap();

        assert_eq!(
            json.options.status,
            vec![
                TouchedStatus::Deleted,
                TouchedStatus::Added,
                TouchedStatus::Modified
            ]
        );
    }
}
