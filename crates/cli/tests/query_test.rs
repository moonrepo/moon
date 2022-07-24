use moon_cli::enums::TouchedStatus;
use moon_cli::queries::projects::QueryProjectsResult;
use moon_cli::queries::touched_files::QueryTouchedFilesResult;
use moon_utils::string_vec;
use moon_utils::test::{
    create_moon_command_in, create_sandbox, get_assert_output, run_git_command,
};

mod projects {
    use super::*;

    #[test]
    fn returns_all_by_default() {
        let fixture = create_sandbox("projects");

        let assert = create_moon_command_in(fixture.path())
            .arg("query")
            .arg("projects")
            .assert();

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
                "tasks",
                "ts"
            ]
        );
    }

    #[test]
    fn can_filter_by_id() {
        let fixture = create_sandbox("projects");

        let assert = create_moon_command_in(fixture.path())
            .arg("query")
            .arg("projects")
            .args(["--id", "ba(r|z)"])
            .assert();

        let json: QueryProjectsResult = serde_json::from_str(&get_assert_output(&assert)).unwrap();
        let ids: Vec<String> = json.projects.iter().map(|p| p.id.clone()).collect();

        assert_eq!(ids, string_vec!["bar", "baz"]);
        assert_eq!(json.options.id.unwrap(), "ba(r|z)".to_string());
    }

    #[test]
    fn can_filter_by_source() {
        let fixture = create_sandbox("projects");

        let assert = create_moon_command_in(fixture.path())
            .arg("query")
            .arg("projects")
            .args(["--source", "config$"])
            .assert();

        let json: QueryProjectsResult = serde_json::from_str(&get_assert_output(&assert)).unwrap();
        let ids: Vec<String> = json.projects.iter().map(|p| p.id.clone()).collect();

        assert_eq!(ids, string_vec!["emptyConfig", "noConfig"]);
        assert_eq!(json.options.source.unwrap(), "config$".to_string());
    }

    #[test]
    fn can_filter_by_tasks() {
        let fixture = create_sandbox("projects");

        let assert = create_moon_command_in(fixture.path())
            .arg("query")
            .arg("projects")
            .args(["--tasks", "lint"])
            .assert();

        let json: QueryProjectsResult = serde_json::from_str(&get_assert_output(&assert)).unwrap();
        let ids: Vec<String> = json.projects.iter().map(|p| p.id.clone()).collect();

        assert_eq!(ids, string_vec!["tasks"]);
        assert_eq!(json.options.tasks.unwrap(), "lint".to_string());
    }

    #[test]
    fn can_filter_by_language() {
        let fixture = create_sandbox("projects");

        let assert = create_moon_command_in(fixture.path())
            .arg("query")
            .arg("projects")
            .args(["--language", "java|bash"])
            .assert();

        let json: QueryProjectsResult = serde_json::from_str(&get_assert_output(&assert)).unwrap();
        let ids: Vec<String> = json.projects.iter().map(|p| p.id.clone()).collect();

        assert_eq!(ids, string_vec!["bash", "basic", "foo", "js"]);
        assert_eq!(json.options.language.unwrap(), "java|bash".to_string());
    }

    #[test]
    fn can_filter_by_type() {
        let fixture = create_sandbox("projects");

        let assert = create_moon_command_in(fixture.path())
            .arg("query")
            .arg("projects")
            .args(["--type", "app"])
            .assert();

        let json: QueryProjectsResult = serde_json::from_str(&get_assert_output(&assert)).unwrap();
        let ids: Vec<String> = json.projects.iter().map(|p| p.id.clone()).collect();

        assert_eq!(ids, string_vec!["advanced", "foo", "ts"]);
        assert_eq!(json.options.type_of.unwrap(), "app".to_string());
    }
}

mod touched_files {
    use super::*;
    use moon_utils::test::create_sandbox_with_git;

    #[test]
    fn can_change_options() {
        let fixture = create_sandbox_with_git("cases");

        run_git_command(fixture.path(), |cmd| {
            cmd.args(["checkout", "-b", "branch"]);
        });

        let assert = create_moon_command_in(fixture.path())
            .arg("query")
            .arg("touched-files")
            .args([
                "--base", "master", "--head", "branch", "--status", "deleted",
            ])
            .assert();

        let json: QueryTouchedFilesResult =
            serde_json::from_str(&get_assert_output(&assert)).unwrap();

        assert_eq!(json.options.base, "master".to_string());
        assert_eq!(json.options.head, "branch".to_string());
        assert_eq!(json.options.status, TouchedStatus::Deleted);
        assert!(!json.options.local);
    }
}
