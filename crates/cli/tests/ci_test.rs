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

// A chain of `c:build` -> `b:build` -> `a:build`, where each task
// only has its own file as an input
fn create_chain_sandbox() -> MoonSandbox {
    let sandbox = create_empty_moon_sandbox();
    sandbox.with_default_projects();

    for (id, dep) in [("a", None), ("b", Some("a")), ("c", Some("b"))] {
        let mut config =
            format!("tasks:\n  build:\n    command: 'noop'\n    inputs: ['{id}.txt']\n");

        if let Some(dep) = dep {
            config.push_str(&format!("    deps: ['{dep}:build']\n"));
        }

        sandbox.create_file(format!("{id}/moon.yml"), config);
        sandbox.create_file(format!("{id}/{id}.txt"), "");
    }

    sandbox.enable_git();
    sandbox
}

mod ci {
    use super::*;

    mod dependents {
        use super::*;

        #[test]
        fn doesnt_run_direct_dependents_by_default() {
            let sandbox = create_chain_sandbox();

            commit_files(&sandbox, ["a/a.txt"]);

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("ci").arg("a:build").arg("--base").arg("master");
            });

            let output = assert.output();

            assert!(predicate::str::contains("RunTask(a:build)").eval(&output));
            assert!(!predicate::str::contains("RunTask(b:build)").eval(&output));

            assert.success();
        }

        #[test]
        fn runs_direct_dependents_when_requested() {
            let sandbox = create_chain_sandbox();

            commit_files(&sandbox, ["a/a.txt"]);

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("ci")
                    .arg("a:build")
                    .arg("--base")
                    .arg("master")
                    .arg("--dependents")
                    .arg("direct");
            });

            let output = assert.output();

            assert!(predicate::str::contains("RunTask(a:build)").eval(&output));
            assert!(predicate::str::contains("RunTask(b:build)").eval(&output));
            assert!(!predicate::str::contains("RunTask(c:build)").eval(&output));

            assert.success();
        }

        #[test]
        fn runs_affected_dependents_within_targets() {
            let sandbox = create_chain_sandbox();

            commit_files(&sandbox, ["a/a.txt"]);

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("ci").arg("--base").arg("master");
            });

            let output = assert.output();

            assert!(predicate::str::contains("RunTask(a:build)").eval(&output));
            assert!(predicate::str::contains("RunTask(b:build)").eval(&output));
            assert!(predicate::str::contains("RunTask(c:build)").eval(&output));

            assert.success();
        }

        #[test]
        fn can_limit_affected_depth() {
            let sandbox = create_chain_sandbox();

            commit_files(&sandbox, ["a/a.txt"]);

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("ci")
                    .arg("--base")
                    .arg("master")
                    .arg("--affected-depth")
                    .arg("none");
            });

            let output = assert.output();

            assert!(predicate::str::contains("RunTask(a:build)").eval(&output));
            assert!(!predicate::str::contains("RunTask(b:build)").eval(&output));
            assert!(!predicate::str::contains("RunTask(c:build)").eval(&output));

            assert.success();
        }
    }

    mod persistent {
        use super::*;

        #[test]
        fn doesnt_run_on_its_own() {
            let sandbox = create_persistent_sandbox();

            commit_files(&sandbox, ["app/server.txt"]);

            // Don't affect `app:e2e` through its dependency on `app:server`
            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("ci")
                    .arg("--base")
                    .arg("master")
                    .arg("--affected-depth")
                    .arg("none");
            });

            let output = assert.output();

            assert!(predicate::str::contains("No tasks affected by changed files").eval(&output));
            assert!(!predicate::str::contains("RunPersistentTask(app:server)").eval(&output));

            assert.success();
        }

        #[test]
        fn runs_as_a_dependency_of_an_affected_dependent() {
            let sandbox = create_persistent_sandbox();

            commit_files(&sandbox, ["app/server.txt"]);

            // `app:e2e` is affected through its dependency on `app:server`
            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("ci").arg("--base").arg("master");
            });

            let output = assert.output();

            assert!(predicate::str::contains("RunTask(app:e2e)").eval(&output));
            assert!(predicate::str::contains("RunPersistentTask(app:server)").eval(&output));

            assert.success();
        }

        #[test]
        fn doesnt_run_as_a_dependent() {
            let sandbox = create_persistent_sandbox();

            commit_files(&sandbox, ["app/build.txt", "app/server.txt"]);

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("ci")
                    .arg("app:build")
                    .arg("--base")
                    .arg("master")
                    .arg("--dependents")
                    .arg("direct");
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
