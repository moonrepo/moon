mod utils;

use moon_test_utils2::predicates::prelude::*;
use utils::create_pipeline_sandbox;

mod check {
    use super::*;

    #[test]
    fn runs_tasks_in_one_project() {
        let sandbox = create_pipeline_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("check").arg("check-a");
        });

        assert.success().stdout(
            predicate::str::contains("check-a:build").and(predicate::str::contains("check-a:test")),
        );
    }

    #[test]
    fn runs_tasks_in_one_project_using_cwd() {
        let sandbox = create_pipeline_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.current_dir(sandbox.path().join("check-a"))
                .arg("check")
                .arg("--closest");
        });

        assert.success().stdout(
            predicate::str::contains("check-a:build").and(predicate::str::contains("check-a:test")),
        );
    }

    #[test]
    fn runs_tasks_in_many_projects() {
        let sandbox = create_pipeline_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("check").arg("check-a").arg("check-b");
        });

        assert.success().stdout(
            predicate::str::contains("check-a:build")
                .and(predicate::str::contains("check-a:test"))
                .and(predicate::str::contains("check-b:test")),
        );
    }

    #[test]
    fn doesnt_run_internal_tasks() {
        let sandbox = create_pipeline_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("check").arg("check-a");
        });

        assert
            .success()
            .stdout(predicate::str::contains("check-a:internal").not());
    }
}
