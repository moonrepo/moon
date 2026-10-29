mod utils;

use moon_test_utils::{MoonSandbox, create_empty_moon_sandbox, predicates::prelude::*};
use rustc_hash::FxHashSet;
use starbase_utils::json::serde_json;
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

    mod query_pre_exec {
        use super::*;

        #[test]
        fn exports_resolved_graph_without_running_pipeline() {
            let sandbox = create_chain_sandbox();

            sandbox.create_file(
                "c/moon.yml",
                r#"
tasks:
  build:
    command: 'touch export-marker.txt'
    inputs: ['c.txt']
    deps: ['b:build']
    env:
      EXPORT_SECRET: 'do-not-export'
"#,
            );

            commit_files(&sandbox, ["c/c.txt"]);

            let run = || {
                sandbox.run_bin(|cmd| {
                    cmd.args(["query", "pre-exec"])
                        .arg("c:build")
                        .arg("--base")
                        .arg("master");
                })
            };

            let first = run();
            let graph: serde_json::Value = serde_json::from_str(first.stdout().trim()).unwrap();
            assert_eq!(graph["schemaVersion"], 1);
            assert_eq!(graph["primaryTargets"], serde_json::json!(["c:build"]));

            let actions = graph["actions"].as_array().unwrap();
            let task_targets = actions
                .iter()
                .filter_map(|action| action["target"].as_str())
                .collect::<Vec<_>>();
            assert_eq!(task_targets, ["a:build", "b:build", "c:build"]);

            let ids = actions
                .iter()
                .map(|action| action["id"].as_str().unwrap())
                .collect::<FxHashSet<_>>();
            let edges = graph["edges"].as_array().unwrap();
            assert!(!edges.is_empty());
            for edge in edges {
                assert!(ids.contains(edge["from"].as_str().unwrap()));
                assert!(ids.contains(edge["to"].as_str().unwrap()));
            }
            let task_id = |target| {
                actions
                    .iter()
                    .find(|action| action["target"] == target)
                    .unwrap()["id"]
                    .as_str()
                    .unwrap()
            };
            assert!(edges.iter().any(|edge| {
                edge["from"] == task_id("c:build") && edge["to"] == task_id("b:build")
            }));

            assert!(!first.stdout().contains("do-not-export"));
            assert!(!first.stdout().contains("export-marker.txt"));

            assert!(!sandbox.path().join(".moon/cache/ciReport.json").exists());
            assert!(!sandbox.path().join("c/export-marker.txt").exists());
            first.success();

            let second = run();
            let second_graph: serde_json::Value =
                serde_json::from_str(second.stdout().trim()).unwrap();
            assert_eq!(graph, second_graph);
            second.success();

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("ci").arg("c:build").arg("--base").arg("master");
                })
                .success();
            let report: serde_json::Value = serde_json::from_slice(
                &std::fs::read(sandbox.path().join(".moon/cache/ciReport.json")).unwrap(),
            )
            .unwrap();
            let executed_tasks = report["actions"]
                .as_array()
                .unwrap()
                .iter()
                .filter_map(|action| action["label"].as_str())
                .filter(|label| label.starts_with("RunTask("))
                .collect::<FxHashSet<_>>();
            let exported_tasks = actions
                .iter()
                .filter(|action| action["kind"] == "run-task")
                .filter_map(|action| action["label"].as_str())
                .collect::<FxHashSet<_>>();
            assert_eq!(exported_tasks, executed_tasks);
        }

        #[test]
        fn exports_empty_graph_when_no_tasks_are_affected() {
            let sandbox = create_chain_sandbox();

            commit_files(&sandbox, ["unrelated.txt"]);

            let assert = sandbox.run_bin(|cmd| {
                cmd.args(["query", "pre-exec"])
                    .arg("a:build")
                    .arg("--base")
                    .arg("master");
            });
            let graph: serde_json::Value = serde_json::from_str(assert.stdout().trim()).unwrap();
            assert_eq!(graph["primaryTargets"], serde_json::json!([]));
            assert_eq!(graph["actions"], serde_json::json!([]));
            assert_eq!(graph["edges"], serde_json::json!([]));
            assert.success();
        }

        #[test]
        fn uses_ci_dependent_selection() {
            let sandbox = create_chain_sandbox();

            commit_files(&sandbox, ["a/a.txt"]);

            let assert = sandbox.run_bin(|cmd| {
                cmd.args(["query", "pre-exec"])
                    .arg("a:build")
                    .arg("--base")
                    .arg("master")
                    .arg("--dependents")
                    .arg("direct");
            });
            let graph: serde_json::Value = serde_json::from_str(assert.stdout().trim()).unwrap();
            assert_eq!(
                graph["primaryTargets"],
                serde_json::json!(["a:build", "b:build"])
            );
            let task_targets = graph["actions"]
                .as_array()
                .unwrap()
                .iter()
                .filter_map(|action| action["target"].as_str())
                .collect::<Vec<_>>();
            assert_eq!(task_targets, ["a:build", "b:build"]);
            assert.success();
        }

        #[test]
        fn uses_plan_targets_instead_of_cli_targets() {
            let sandbox = create_chain_sandbox();
            sandbox.create_file("plan.json", r#"{ "targets": ["b:build"] }"#);
            commit_files(&sandbox, ["b/b.txt"]);

            let assert = sandbox.run_bin(|cmd| {
                cmd.args(["query", "pre-exec", "c:build"])
                    .arg("--plan")
                    .arg("plan.json")
                    .arg("--base")
                    .arg("master");
            });
            let graph: serde_json::Value = serde_json::from_str(assert.stdout().trim()).unwrap();
            assert_eq!(graph["requestedTargets"], serde_json::json!(["b:build"]));
            assert_eq!(graph["primaryTargets"], serde_json::json!(["b:build"]));
            assert.success();
        }

        #[test]
        fn partitions_primary_targets_before_exporting_the_graph() {
            let sandbox = create_chain_sandbox();
            commit_files(&sandbox, ["a/a.txt", "b/b.txt", "c/c.txt"]);

            let mut primary_targets = FxHashSet::default();
            for job in [0, 1] {
                let assert = sandbox.run_bin(|cmd| {
                    cmd.args(["query", "pre-exec"])
                        .arg("--base")
                        .arg("master")
                        .arg("--job")
                        .arg(job.to_string())
                        .arg("--job-total")
                        .arg("2");
                });
                let graph: serde_json::Value =
                    serde_json::from_str(assert.stdout().trim()).unwrap();
                for target in graph["primaryTargets"].as_array().unwrap() {
                    assert!(primary_targets.insert(target.as_str().unwrap().to_owned()));
                }
                assert.success();
            }

            assert_eq!(
                primary_targets,
                FxHashSet::from_iter(["a:build", "b:build", "c:build"].map(str::to_owned))
            );
        }
    }

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
