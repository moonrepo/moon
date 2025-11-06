mod utils;

use moon_app::queries::projects::QueryProjectsResult;
use moon_common::is_ci;
use moon_test_utils2::MoonSandbox;
use starbase_utils::json::serde_json;
use utils::{change_files, create_query_sandbox};

fn change_many_files(sandbox: &MoonSandbox) {
    change_files(
        sandbox,
        [
            "advanced/file.txt",
            "metadata/file.txt",
            "no-config/file.txt",
        ],
    );
}

mod query_projects {
    use super::*;

    #[test]
    fn returns_all_by_default() {
        let sandbox = create_query_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("query").arg("projects");
        });

        let json: QueryProjectsResult = serde_json::from_str(assert.stdout().trim()).unwrap();
        let mut ids: Vec<String> = json.projects.iter().map(|p| p.id.to_string()).collect();

        ids.sort();

        assert_eq!(
            ids,
            [
                "advanced",
                "basic",
                "dep-bar",
                "dep-baz",
                "dep-foo",
                "empty-config",
                "metadata",
                "no-config",
                "root",
                "tasks",
                "toolchains",
            ]
        );
    }

    #[test]
    fn can_filter_by_affected() {
        let sandbox = create_query_sandbox();

        change_many_files(&sandbox);

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("query").arg("projects").arg("--affected");
        });

        let json: QueryProjectsResult = serde_json::from_str(assert.stdout().trim()).unwrap();
        let ids: Vec<String> = json.projects.iter().map(|p| p.id.to_string()).collect();

        assert_eq!(ids, ["advanced", "metadata", "no-config", "root"]);
        assert!(json.options.affected.is_some());
    }

    #[test]
    fn can_filter_by_affected_via_stdin() {
        let sandbox = create_query_sandbox();

        change_many_files(&sandbox);

        let query = sandbox.run_bin(|cmd| {
            cmd.arg("query").arg("changed-files");

            if !is_ci() {
                cmd.arg("--local");
            }
        });

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("query")
                .arg("projects")
                .arg("--affected")
                .write_stdin(query.stdout());
        });

        let json: QueryProjectsResult = serde_json::from_str(assert.stdout().trim()).unwrap();
        let ids: Vec<String> = json.projects.iter().map(|p| p.id.to_string()).collect();

        assert_eq!(ids, ["advanced", "metadata", "no-config", "root"]);
        assert!(json.options.affected.is_some());
    }

    #[test]
    fn can_include_dependents_for_affected() {
        let sandbox = create_query_sandbox();

        change_many_files(&sandbox);

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("query")
                .arg("projects")
                .arg("--affected")
                .arg("--downstream")
                .arg("deep");
        });

        let json: QueryProjectsResult = serde_json::from_str(assert.stdout().trim()).unwrap();
        let ids: Vec<String> = json.projects.iter().map(|p| p.id.to_string()).collect();

        assert_eq!(ids, ["advanced", "basic", "metadata", "no-config", "root"]);
        assert!(json.options.affected.is_some());
    }

    #[test]
    fn can_filter_by_id() {
        let sandbox = create_query_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("query").arg("projects").args(["--id", "ba(r|z)"]);
        });

        let json: QueryProjectsResult = serde_json::from_str(assert.stdout().trim()).unwrap();
        let ids: Vec<String> = json.projects.iter().map(|p| p.id.to_string()).collect();

        assert_eq!(ids, ["dep-bar", "dep-baz"]);
        assert_eq!(json.options.id.unwrap(), "ba(r|z)".to_string());
    }

    #[test]
    fn can_filter_by_source() {
        let sandbox = create_query_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("query")
                .arg("projects")
                .args(["--source", "config$"]);
        });

        let json: QueryProjectsResult = serde_json::from_str(assert.stdout().trim()).unwrap();
        let ids: Vec<String> = json.projects.iter().map(|p| p.id.to_string()).collect();

        assert_eq!(ids, ["empty-config", "no-config"]);
        assert_eq!(json.options.source.unwrap(), "config$".to_string());
    }

    #[test]
    fn can_filter_by_tags() {
        let sandbox = create_query_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("query").arg("projects").args(["--tags", "react"]);
        });

        let json: QueryProjectsResult = serde_json::from_str(assert.stdout().trim()).unwrap();
        let ids: Vec<String> = json.projects.iter().map(|p| p.id.to_string()).collect();

        assert_eq!(ids, ["advanced", "dep-foo"]);
        assert_eq!(json.options.tags.unwrap(), "react".to_string());

        // Multiple
        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("query")
                .arg("projects")
                .args(["--tags", "react|vue"]);
        });

        let json: QueryProjectsResult = serde_json::from_str(assert.stdout().trim()).unwrap();
        let ids: Vec<String> = json.projects.iter().map(|p| p.id.to_string()).collect();

        assert_eq!(ids, ["advanced", "basic", "dep-foo"]);
        assert_eq!(json.options.tags.unwrap(), "react|vue".to_string());
    }

    #[test]
    fn can_filter_by_tasks() {
        let sandbox = create_query_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("query").arg("projects").args(["--tasks", "lint"]);
        });

        let json: QueryProjectsResult = serde_json::from_str(assert.stdout().trim()).unwrap();
        let ids: Vec<String> = json.projects.iter().map(|p| p.id.to_string()).collect();

        assert_eq!(ids, ["tasks"]);
        assert_eq!(json.options.tasks.unwrap(), "lint".to_string());
    }

    #[test]
    fn can_filter_by_language() {
        let sandbox = create_query_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("query")
                .arg("projects")
                .args(["--language", "java|bash"]);
        });

        let json: QueryProjectsResult = serde_json::from_str(assert.stdout().trim()).unwrap();
        let ids: Vec<String> = json.projects.iter().map(|p| p.id.to_string()).collect();

        assert_eq!(ids, ["basic", "dep-foo"]);
        assert_eq!(json.options.language.unwrap(), "java|bash".to_string());
    }

    #[test]
    fn can_filter_by_type() {
        let sandbox = create_query_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("query").arg("projects").args(["--layer", "app"]);
        });

        let json: QueryProjectsResult = serde_json::from_str(assert.stdout().trim()).unwrap();
        let ids: Vec<String> = json.projects.iter().map(|p| p.id.to_string()).collect();

        assert_eq!(ids, ["advanced", "dep-foo"]);
        assert_eq!(json.options.layer.unwrap(), "app".to_string());
    }

    #[test]
    fn can_filter_by_stack() {
        let sandbox = create_query_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("query")
                .arg("projects")
                .args(["--stack", "frontend"]);
        });

        let json: QueryProjectsResult = serde_json::from_str(assert.stdout().trim()).unwrap();
        let ids: Vec<String> = json.projects.iter().map(|p| p.id.to_string()).collect();

        assert_eq!(ids, ["advanced"]);
        assert_eq!(json.options.stack.unwrap(), "frontend".to_string());
    }

    mod mql {
        use super::*;

        #[test]
        fn can_filter_with_query() {
            let sandbox = create_query_sandbox();

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("query").arg("projects").arg("project~dep-ba{r,z}");
            });

            let json: QueryProjectsResult = serde_json::from_str(assert.stdout().trim()).unwrap();
            let ids: Vec<String> = json.projects.iter().map(|p| p.id.to_string()).collect();

            assert_eq!(ids, ["dep-bar", "dep-baz"]);
            assert_eq!(
                json.options.query.unwrap(),
                "project~dep-ba{r,z}".to_string()
            );
        }

        #[test]
        fn can_filter_by_affected_with_query() {
            let sandbox = create_query_sandbox();

            change_many_files(&sandbox);

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("query")
                    .arg("projects")
                    .arg("project~*config")
                    .arg("--affected");
            });

            let json: QueryProjectsResult = serde_json::from_str(assert.stdout().trim()).unwrap();
            let ids: Vec<String> = json.projects.iter().map(|p| p.id.to_string()).collect();

            assert_eq!(ids, ["no-config"]);
            assert!(json.options.affected.is_some());
        }
    }
}
