use moon_cli::enums::TouchedStatus;
use moon_cli::queries::projects::QueryProjectsResult;
use moon_cli::queries::touched_files::QueryTouchedFilesResult;
use moon_test_utils::{
    create_sandbox_with_config, get_assert_output, get_assert_stdout_output,
    get_cases_fixture_configs, get_projects_fixture_configs, Sandbox,
};
use moon_utils::{is_ci, string_vec};
use std::fs;

fn change_branch(sandbox: &Sandbox) {
    sandbox.run_git(|cmd| {
        cmd.args(["checkout", "-b", "branch"]);
    });
}

fn touch_file(sandbox: &Sandbox) {
    sandbox.create_file("advanced/file", "contents");

    // CI uses `git diff` while local uses `git status`
    if is_ci() {
        change_branch(sandbox);

        sandbox.run_git(|cmd| {
            cmd.args(["add", "advanced/file"]);
        });

        sandbox.run_git(|cmd| {
            cmd.args(["commit", "-m", "Touch"]);
        });
    }
}

mod projects {
    use super::*;

    #[test]
    fn returns_all_by_default() {
        let (workspace_config, toolchain_config, projects_config) = get_projects_fixture_configs();

        let sandbox = create_sandbox_with_config(
            "projects",
            Some(&workspace_config),
            Some(&toolchain_config),
            Some(&projects_config),
        );

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("query").arg("projects");
        });

        let json: QueryProjectsResult = serde_json::from_str(&get_assert_output(&assert)).unwrap();
        let ids: Vec<String> = json.projects.iter().map(|p| p.id.clone()).collect();

        assert_eq!(
            ids,
            string_vec![
                "advanced",
                "bar",
                "bash",
                "basic",
                "baz",
                "emptyConfig",
                "foo",
                "js",
                "noConfig",
                "platforms",
                "tasks",
                "ts"
            ]
        );
    }

    #[test]
    fn can_filter_by_affected() {
        let (workspace_config, toolchain_config, projects_config) = get_projects_fixture_configs();

        let sandbox = create_sandbox_with_config(
            "projects",
            Some(&workspace_config),
            Some(&toolchain_config),
            Some(&projects_config),
        );
        sandbox.enable_git();

        touch_file(&sandbox);

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("query").arg("projects").arg("--affected");
        });

        let json: QueryProjectsResult = serde_json::from_str(&get_assert_output(&assert)).unwrap();
        let ids: Vec<String> = json.projects.iter().map(|p| p.id.clone()).collect();

        assert_eq!(ids, string_vec!["advanced"]);
        assert!(json.options.affected);
    }

    #[test]
    fn can_filter_by_affected_via_stdin() {
        let (workspace_config, toolchain_config, projects_config) = get_projects_fixture_configs();

        let sandbox = create_sandbox_with_config(
            "projects",
            Some(&workspace_config),
            Some(&toolchain_config),
            Some(&projects_config),
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
                .write_stdin(get_assert_stdout_output(&query));
        });

        let json: QueryProjectsResult = serde_json::from_str(&get_assert_output(&assert)).unwrap();
        let ids: Vec<String> = json.projects.iter().map(|p| p.id.clone()).collect();

        assert_eq!(ids, string_vec!["advanced"]);
        assert!(json.options.affected);
    }

    #[test]
    fn can_filter_by_id() {
        let (workspace_config, toolchain_config, projects_config) = get_projects_fixture_configs();

        let sandbox = create_sandbox_with_config(
            "projects",
            Some(&workspace_config),
            Some(&toolchain_config),
            Some(&projects_config),
        );

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("query").arg("projects").args(["--id", "ba(r|z)"]);
        });

        let json: QueryProjectsResult = serde_json::from_str(&get_assert_output(&assert)).unwrap();
        let ids: Vec<String> = json.projects.iter().map(|p| p.id.clone()).collect();

        assert_eq!(ids, string_vec!["bar", "baz"]);
        assert_eq!(json.options.id.unwrap(), "ba(r|z)".to_string());
    }

    #[test]
    fn can_filter_by_source() {
        let (workspace_config, toolchain_config, projects_config) = get_projects_fixture_configs();

        let sandbox = create_sandbox_with_config(
            "projects",
            Some(&workspace_config),
            Some(&toolchain_config),
            Some(&projects_config),
        );

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("query")
                .arg("projects")
                .args(["--source", "config$"]);
        });

        let json: QueryProjectsResult = serde_json::from_str(&get_assert_output(&assert)).unwrap();
        let ids: Vec<String> = json.projects.iter().map(|p| p.id.clone()).collect();

        assert_eq!(ids, string_vec!["emptyConfig", "noConfig"]);
        assert_eq!(json.options.source.unwrap(), "config$".to_string());
    }

    #[test]
    fn can_filter_by_tasks() {
        let (workspace_config, toolchain_config, projects_config) = get_projects_fixture_configs();

        let sandbox = create_sandbox_with_config(
            "projects",
            Some(&workspace_config),
            Some(&toolchain_config),
            Some(&projects_config),
        );

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("query").arg("projects").args(["--tasks", "lint"]);
        });

        let json: QueryProjectsResult = serde_json::from_str(&get_assert_output(&assert)).unwrap();
        let ids: Vec<String> = json.projects.iter().map(|p| p.id.clone()).collect();

        assert_eq!(ids, string_vec!["tasks"]);
        assert_eq!(json.options.tasks.unwrap(), "lint".to_string());
    }

    #[test]
    fn can_filter_by_language() {
        let (workspace_config, toolchain_config, projects_config) = get_projects_fixture_configs();

        let sandbox = create_sandbox_with_config(
            "projects",
            Some(&workspace_config),
            Some(&toolchain_config),
            Some(&projects_config),
        );

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("query")
                .arg("projects")
                .args(["--language", "java|bash"]);
        });

        let json: QueryProjectsResult = serde_json::from_str(&get_assert_output(&assert)).unwrap();
        let ids: Vec<String> = json.projects.iter().map(|p| p.id.clone()).collect();

        assert_eq!(ids, string_vec!["bash", "basic", "foo", "js"]);
        assert_eq!(json.options.language.unwrap(), "java|bash".to_string());
    }

    #[test]
    fn can_filter_by_type() {
        let (workspace_config, toolchain_config, projects_config) = get_projects_fixture_configs();

        let sandbox = create_sandbox_with_config(
            "projects",
            Some(&workspace_config),
            Some(&toolchain_config),
            Some(&projects_config),
        );

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("query").arg("projects").args(["--type", "app"]);
        });

        let json: QueryProjectsResult = serde_json::from_str(&get_assert_output(&assert)).unwrap();
        let ids: Vec<String> = json.projects.iter().map(|p| p.id.clone()).collect();

        assert_eq!(ids, string_vec!["advanced", "foo", "ts"]);
        assert_eq!(json.options.type_of.unwrap(), "app".to_string());
    }
}

mod touched_files {
    use super::*;

    #[test]
    fn can_change_options() {
        let (workspace_config, toolchain_config, projects_config) = get_cases_fixture_configs();

        let sandbox = create_sandbox_with_config(
            "cases",
            Some(&workspace_config),
            Some(&toolchain_config),
            Some(&projects_config),
        );
        sandbox.enable_git();

        change_branch(&sandbox);

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("query").arg("touched-files").args([
                "--base", "master", "--head", "branch", "--status", "deleted",
            ]);
        });

        let json: QueryTouchedFilesResult =
            serde_json::from_str(&get_assert_output(&assert)).unwrap();

        assert_eq!(json.options.base, "master".to_string());
        assert_eq!(json.options.head, "branch".to_string());
        assert_eq!(json.options.status, TouchedStatus::Deleted);
        assert!(!json.options.local);
    }
}
