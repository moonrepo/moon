use moon_test_utils::{MoonSandbox, create_empty_sandbox, predicates::prelude::*};
use starbase_sandbox::assert_snapshot;
use std::fs;

fn read_file(sandbox: &MoonSandbox, path: &str) -> String {
    fs::read_to_string(sandbox.path().join(path)).unwrap()
}

fn run_migrate(sandbox: &MoonSandbox) -> starbase_sandbox::SandboxAssert<'_> {
    sandbox.run_bin(|cmd| {
        cmd.arg("migrate").arg("v3").arg("--yes");
    })
}

mod migrate_v3 {
    use super::*;

    #[test]
    fn migrates_workspace_settings() {
        let sandbox = create_empty_sandbox();
        sandbox.create_file(
            ".moon/workspace.yml",
            r"
projects: ['*']
unstable_daemon: false
generator:
  templates: ['./templates']
cache:
  unstable_sharedWorktreeCache: true
experiments:
  nativeFileHashing: true
  asyncAffectedTracking: true
remote:
  host: 'grpcs://cache.example.com'
  cache:
    verifyIntegrity: true
vcs:
  provider: bitbucket-legacy
  defaultBranch: main
",
        );

        run_migrate(&sandbox).success();

        assert_snapshot!(read_file(&sandbox, ".moon/workspace.yml"));
    }

    #[test]
    fn keeps_new_setting_when_both_are_defined() {
        let sandbox = create_empty_sandbox();
        sandbox.create_file(
            ".moon/workspace.yml",
            r"
cache:
  cas:
    verifyIntegrity: false
remote:
  host: 'grpcs://cache.example.com'
  cache:
    compression: zstd
    verifyIntegrity: true
",
        );

        run_migrate(&sandbox).success();

        assert_snapshot!(read_file(&sandbox, ".moon/workspace.yml"));
    }

    #[test]
    fn warns_when_async_graph_building_was_disabled() {
        let sandbox = create_empty_sandbox();
        sandbox.create_file(
            ".moon/workspace.yml",
            r"
experiments:
  asyncGraphBuilding: false
",
        );

        run_migrate(&sandbox)
            .success()
            .stderr(predicate::str::contains(
                "Removed the asyncGraphBuilding experiment",
            ));

        assert_eq!(read_file(&sandbox, ".moon/workspace.yml"), "{}\n");
    }

    #[test]
    fn migrates_workspace_in_toml() {
        let sandbox = create_empty_sandbox();
        sandbox.create_file(
            ".moon/workspace.toml",
            r#"
projects = ["*"]

[generator]
templates = ["./templates"]

[remote]
host = "grpcs://cache.example.com"

[remote.cache]
verifyIntegrity = true
"#,
        );

        run_migrate(&sandbox).success();

        assert_snapshot!(read_file(&sandbox, ".moon/workspace.toml"));

        // Ensure the migrated config can still be loaded
        sandbox
            .run_bin(|cmd| {
                cmd.arg("templates");
            })
            .success();
    }

    #[test]
    fn migrates_workspace_in_hcl() {
        let sandbox = create_empty_sandbox();
        sandbox.create_file(
            ".moon/workspace.hcl",
            r#"
projects = ["*"]

generator {
	templates = ["./templates"]
}
"#,
        );

        run_migrate(&sandbox).success();

        assert_snapshot!(read_file(&sandbox, ".moon/workspace.hcl"));

        // Ensure the migrated config can still be loaded
        sandbox
            .run_bin(|cmd| {
                cmd.arg("templates");
            })
            .success();
    }

    #[test]
    fn migrates_tasks_config_task_options() {
        let sandbox = create_empty_sandbox();
        sandbox.create_file(
            ".moon/tasks/node.yml",
            r"
inheritedBy:
  toolchain: node
taskOptions:
  cache: false
  allowFailure: true
  runInCI: false
tasks:
  lint:
    command: eslint
    options:
      expectFailure: true
  test:
    command: jest
    options:
      allowFailure: true
      expectFailure: true
  format:
    command: prettier
    options:
      allowFailure: true
      onFailure: expect
",
        );

        run_migrate(&sandbox).success();

        assert_snapshot!(read_file(&sandbox, ".moon/tasks/node.yml"));
    }

    #[test]
    fn migrates_project_task_options() {
        let sandbox = create_empty_sandbox();
        sandbox.create_file(
            "packages/app/moon.yml",
            r"
language: typescript
tasks:
  build:
    command: vite build
    options:
      allowFailure: false
      cache: true
",
        );

        run_migrate(&sandbox)
            .success()
            .stderr(predicate::str::contains(
                "Removed allowFailure: false from task build",
            ));

        assert_snapshot!(read_file(&sandbox, "packages/app/moon.yml"));
    }

    #[test]
    fn migrates_project_in_json() {
        let sandbox = create_empty_sandbox();
        sandbox.create_file(
            "packages/app/moon.json",
            r#"{
  // Comments are supported
  "language": "typescript",
  "taskOptions": { "expectFailure": true },
  "tasks": { "build": { "command": "vite build", "options": { "allowFailure": true } } }
}
"#,
        );

        run_migrate(&sandbox).success();

        assert_snapshot!(read_file(&sandbox, "packages/app/moon.json"));
    }

    #[test]
    fn doesnt_rewrite_files_without_changes() {
        let sandbox = create_empty_sandbox();
        let workspace = "# Comment\nprojects: ['*']\npipeline:\n  logRunningCommand: true\n";
        let project = "# Comment\ntasks:\n  build:\n    command: 'vite build'\n    options:\n      onFailure: allow\n";

        sandbox.create_file(".moon/workspace.yml", workspace);
        sandbox.create_file("packages/app/moon.yml", project);

        run_migrate(&sandbox).success();

        assert_eq!(read_file(&sandbox, ".moon/workspace.yml"), workspace);
        assert_eq!(read_file(&sandbox, "packages/app/moon.yml"), project);
    }

    #[test]
    fn skips_nested_node_modules() {
        let sandbox = create_empty_sandbox();
        let project = "tasks:\n  build:\n    options:\n      allowFailure: true\n";

        sandbox.create_file("packages/app/node_modules/dep/moon.yml", project);

        run_migrate(&sandbox).success();

        assert_eq!(
            read_file(&sandbox, "packages/app/node_modules/dep/moon.yml"),
            project
        );
    }

    #[test]
    fn warns_for_pkl_files() {
        let sandbox = create_empty_sandbox();
        sandbox.create_file(".moon/workspace.pkl", "generator {}\n");

        run_migrate(&sandbox)
            .success()
            .stderr(predicate::str::contains(
                "Pkl based configuration files cannot be automatically migrated",
            ));

        assert_eq!(read_file(&sandbox, ".moon/workspace.pkl"), "generator {}\n");
    }

    #[test]
    fn skips_files_that_fail_to_parse() {
        let sandbox = create_empty_sandbox();
        sandbox.create_file("packages/a/moon.yml", "tasks: [\n");
        sandbox.create_file(
            "packages/b/moon.yml",
            "tasks:\n  build:\n    options:\n      allowFailure: true\n",
        );

        run_migrate(&sandbox)
            .success()
            .stderr(predicate::str::contains(
                "Failed to parse configuration file, skipping migration",
            ));

        assert_eq!(read_file(&sandbox, "packages/a/moon.yml"), "tasks: [\n");
        assert_snapshot!(read_file(&sandbox, "packages/b/moon.yml"));
    }

    #[test]
    fn migrated_configs_can_be_loaded() {
        let sandbox = create_empty_sandbox();
        sandbox.create_file(
            ".moon/workspace.yml",
            r"
projects: ['packages/*']
generator:
  templates: ['./templates']
experiments:
  nativeFileHashing: true
",
        );
        sandbox.create_file(
            ".moon/tasks/all.yml",
            "taskOptions:\n  allowFailure: true\n",
        );
        sandbox.create_file(
            "packages/app/moon.yml",
            "tasks:\n  build:\n    command: noop\n    options:\n      expectFailure: true\n",
        );

        run_migrate(&sandbox).success();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("task").arg("app:build").arg("--json");
            })
            .success()
            .stdout(predicate::str::contains(r#""onFailure": "expect""#));
    }
}
