mod utils;

use moon_test_utils::{MoonSandbox, create_empty_moon_sandbox, predicates::prelude::*};
use utils::create_pipeline_sandbox;

// The persistent tasks have no command, so that they complete instead
// of running forever, as a failing test would otherwise never finish
fn create_persistent_sandbox() -> MoonSandbox {
    let sandbox = create_empty_moon_sandbox();
    sandbox.with_default_projects();
    sandbox.create_file(
        "app/moon.yml",
        r#"
tasks:
  test:
    command: 'noop'
  watch:
    command: 'noop'
    type: 'build'
    options:
      persistent: true
  watch-tests:
    command: 'noop'
    type: 'test'
    options:
      persistent: true
  server:
    command: 'noop'
    type: 'build'
    options:
      persistent: true
      runInCI: true
  e2e:
    command: 'noop'
    deps:
      - target: 'server'
        type: 'wait'
"#,
    );
    sandbox.create_file(
        "servers/moon.yml",
        r#"
tasks:
  watch:
    command: 'noop'
    type: 'build'
    options:
      persistent: true
"#,
    );
    sandbox.enable_git();
    sandbox
}

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

    // Persistent tasks never complete, so neither would the check
    mod persistent {
        use super::*;

        #[test]
        fn doesnt_run_on_its_own() {
            let sandbox = create_persistent_sandbox();

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("check").arg("app");
            });

            assert.success().stdout(
                predicate::str::contains("app:test")
                    .and(predicate::str::contains("app:watch").not())
                    .and(predicate::str::contains("app:watch-tests").not()),
            );
        }

        #[test]
        fn runs_as_a_dependency() {
            let sandbox = create_persistent_sandbox();

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("check").arg("app");
            });

            assert.success().stdout(
                predicate::str::contains("app:e2e").and(predicate::str::contains("app:server")),
            );
        }

        #[test]
        fn doesnt_run_when_theres_nothing_else_to_run() {
            let sandbox = create_persistent_sandbox();

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("check").arg("servers");
            });

            let output = assert.output();

            assert!(predicate::str::contains("No tasks found").eval(&output));
            assert!(!predicate::str::contains("Tasks: 1 completed").eval(&output));

            assert.failure();
        }
    }
}
