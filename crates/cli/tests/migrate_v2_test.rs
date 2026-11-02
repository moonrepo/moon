use moon_test_utils::{MoonSandbox, create_empty_sandbox, predicates::prelude::*};
use starbase_sandbox::assert_snapshot;
use std::fs;

fn read_file(sandbox: &MoonSandbox, path: &str) -> String {
    fs::read_to_string(sandbox.path().join(path)).unwrap()
}

fn run_migrate(sandbox: &MoonSandbox) -> starbase_sandbox::SandboxAssert<'_> {
    sandbox.run_bin(|cmd| {
        cmd.arg("migrate").arg("v2").arg("--yes");
    })
}

mod migrate_v2 {
    use super::*;

    #[test]
    fn migrates_workspace_settings() {
        let sandbox = create_empty_sandbox();
        sandbox.create_file(
            ".moon/workspace.yml",
            r"
projects: ['*']
codeowners:
  orderBy: project-name
  syncOnRun: true
constraints:
  enforceProjectTypeRelationships: true
docker:
  prune:
    installToolchainDeps: false
  scaffold:
    copyToolchainFiles: true
    include: ['*.json']
experiments:
  fasterGlobWalk: true
extensions:
  example:
    plugin: 'https://example.com/plugin.wasm'
hasher:
  batchSize: 100
  optimization: performance
runner:
  archivableTargets: [':build']
  logRunningCommand: true
unstable_remote:
  host: 'grpcs://cache.example.com'
vcs:
  manager: git
  syncHooks: true
  defaultBranch: main
",
        );

        run_migrate(&sandbox).success();

        assert_snapshot!(read_file(&sandbox, ".moon/workspace.yml"));
        assert_snapshot!("extensions", read_file(&sandbox, ".moon/extensions.yml"));
    }

    #[test]
    fn migrates_toolchain_settings() {
        let sandbox = create_empty_sandbox();
        sandbox.create_file(
            ".moon/toolchain.yml",
            r"
node:
  version: '20.0.0'
  binExecArgs: ['--inspect']
  addEnginesConstraint: true
  packageManager: yarn
  rootPackageOnly: true
  yarn:
    version: '4.0.0'
bun:
  version: '1.0.0'
  dependencyVersionFormat: workspace
python:
  version: '3.12.0'
  rootVenvOnly: true
  uv:
    version: '0.5.0'
unstable_go:
  version: '1.22.0'
rust:
  version: '1.80.0'
",
        );

        run_migrate(&sandbox).success();

        assert!(!sandbox.path().join(".moon/toolchain.yml").exists());
        assert_snapshot!(read_file(&sandbox, ".moon/toolchains.yml"));
    }

    #[test]
    fn migrates_tasks_settings() {
        let sandbox = create_empty_sandbox();
        sandbox.create_file(
            ".moon/tasks.yml",
            r"
tasks:
  lint:
    command: 'eslint . && prettier --check .'
    platform: node
  serve:
    command: 'vite dev'
    local: true
  check:
    command: 'tsc'
    local: false
  watch:
    command: 'vite build --watch'
    preset: watcher
    deps:
      - target: '^:build'
        args: '--mode production'
",
        );
        sandbox.create_file(
            ".moon/tasks/node-application.yml",
            r"
tasks:
  build:
    command: 'vite build'
",
        );
        sandbox.create_file(".moon/tasks/tag-ci.yml", "tasks: {}\n");

        run_migrate(&sandbox).success();

        assert!(!sandbox.path().join(".moon/tasks.yml").exists());
        assert_snapshot!("all", read_file(&sandbox, ".moon/tasks/all.yml"));
        assert_snapshot!(
            "node_application",
            read_file(&sandbox, ".moon/tasks/node-application.yml")
        );
        assert_snapshot!("tag_ci", read_file(&sandbox, ".moon/tasks/tag-ci.yml"));
    }

    #[test]
    fn migrates_project_settings() {
        let sandbox = create_empty_sandbox();
        sandbox.create_file(
            "packages/app/moon.yml",
            r"
type: application
language: typescript
platform: node
project:
  name: 'App'
  description: 'An application'
  metadata:
    team: 'web'
docker:
  scaffold:
    include: ['src/**/*']
toolchain:
  typescript:
    disabled: true
tasks:
  build:
    command: 'vite build --base $projectName'
    platform: node
",
        );

        run_migrate(&sandbox).success();

        assert_snapshot!(read_file(&sandbox, "packages/app/moon.yml"));
    }

    #[test]
    fn keeps_new_setting_when_both_are_defined() {
        let sandbox = create_empty_sandbox();
        sandbox.create_file(
            "packages/app/moon.yml",
            r"
platform: node
toolchain:
  default: bun
",
        );

        run_migrate(&sandbox).success();

        assert_snapshot!(read_file(&sandbox, "packages/app/moon.yml"));
    }

    #[test]
    fn skips_nested_node_modules() {
        let sandbox = create_empty_sandbox();
        let project = "type: library\n";

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
        sandbox.create_file(".moon/workspace.pkl", "runner {}\n");

        run_migrate(&sandbox)
            .success()
            .stderr(predicate::str::contains(
                "Pkl based configuration files cannot be automatically migrated",
            ));

        assert_eq!(read_file(&sandbox, ".moon/workspace.pkl"), "runner {}\n");
    }
}
