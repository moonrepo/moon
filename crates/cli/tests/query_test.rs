use insta::assert_snapshot;
use moon_utils::test::{
    create_fixtures_sandbox, create_moon_command_in, get_assert_output, run_git_command,
};

mod touched_files {
    use super::*;

    #[test]
    fn can_change_options() {
        let fixture = create_fixtures_sandbox("cases");

        run_git_command(fixture.path(), "Failed to create branch", |cmd| {
            cmd.args(["checkout", "-b", "branch"]);
        });

        let assert = create_moon_command_in(fixture.path())
            .arg("query")
            .arg("touched-files")
            .args([
                "--base",
                "master",
                "--head",
                "branch",
                "--status",
                "deleted",
                "--upstream",
            ])
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }
}
