mod utils;

use moon_test_utils::{MoonSandbox, create_empty_moon_sandbox, predicates::prelude::*};
use utils::change_branch;

// The persistent tasks have no command, so that they complete instead
// of running forever, as a failing test would otherwise never finish
fn create_persistent_sandbox() -> MoonSandbox {
    let sandbox = create_empty_moon_sandbox();
    sandbox.with_default_projects();
    sandbox.create_file(
        "app/moon.yml",
        r#"
tasks:
  build:
    command: 'noop'
    inputs:
      - 'build.txt'
  server:
    command: 'noop'
    deps:
      - 'build'
    inputs:
      - 'server.txt'
    options:
      persistent: true
      runInCI: true
  e2e:
    command: 'noop'
    deps:
      - target: 'server'
        type: 'wait'
    inputs:
      - 'e2e.txt'
"#,
    );
    sandbox.enable_git();
    sandbox
}

// `moon ci` compares revisions, and not the working tree
fn commit_files<I: IntoIterator<Item = V>, V: AsRef<str>>(sandbox: &MoonSandbox, files: I) {
    change_branch(sandbox, "branch");

    for file in files {
        sandbox.create_file(file.as_ref(), "contents");
    }

    sandbox.run_git(|cmd| {
        cmd.args(["add", "--all", "."]);
    });

    sandbox.run_git(|cmd| {
        cmd.args(["commit", "-m", "Change"]);
    });
}

mod ci {
    use super::*;

    mod persistent {
        use super::*;

        #[test]
        fn doesnt_run_on_its_own() {
            let sandbox = create_persistent_sandbox();

            commit_files(&sandbox, ["app/server.txt"]);

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("ci").arg("--base").arg("master");
            });

            let output = assert.output();

            assert!(predicate::str::contains("No tasks affected by changed files").eval(&output));
            assert!(!predicate::str::contains("RunPersistentTask(app:server)").eval(&output));

            assert.success();
        }

        #[test]
        fn doesnt_run_as_a_dependent() {
            let sandbox = create_persistent_sandbox();

            commit_files(&sandbox, ["app/build.txt", "app/server.txt"]);

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("ci").arg("app:build").arg("--base").arg("master");
            });

            let output = assert.output();

            assert!(predicate::str::contains("RunTask(app:build)").eval(&output));
            assert!(!predicate::str::contains("RunPersistentTask(app:server)").eval(&output));

            assert.success();
        }

        #[test]
        fn runs_as_a_dependency() {
            let sandbox = create_persistent_sandbox();

            commit_files(&sandbox, ["app/e2e.txt"]);

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("ci").arg("--base").arg("master");
            });

            let output = assert.output();

            assert!(predicate::str::contains("RunTask(app:e2e)").eval(&output));
            assert!(predicate::str::contains("RunPersistentTask(app:server)").eval(&output));

            assert.success();
        }

        #[test]
        fn runs_when_requested() {
            let sandbox = create_persistent_sandbox();

            commit_files(&sandbox, ["app/server.txt"]);

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("ci").arg("app:server").arg("--base").arg("master");
            });

            let output = assert.output();

            assert!(predicate::str::contains("RunPersistentTask(app:server)").eval(&output));

            assert.success();
        }
    }
}
