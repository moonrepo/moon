use moon_config::{NodeProjectAliasFormat, WorkspaceConfig, WorkspaceProjects};
use moon_test_utils::{
    assert_snapshot, create_sandbox_with_config, get_default_toolchain, predicates::str::contains,
    Sandbox,
};
use moon_utils::string_vec;
use std::fs;

fn migrate_sandbox() -> Sandbox {
    let workspace_config = WorkspaceConfig {
        projects: WorkspaceProjects::Globs(string_vec!["package-json/*", "turborepo/*"]),
        ..WorkspaceConfig::default()
    };

    let mut toolchain_config = get_default_toolchain();

    toolchain_config.node.as_mut().unwrap().alias_package_names =
        Some(NodeProjectAliasFormat::NameAndScope);

    create_sandbox_with_config(
        "migrate",
        Some(&workspace_config),
        Some(&toolchain_config),
        None,
    )
}

mod from_package_json {
    use super::*;

    #[test]
    fn dirty_repository_raises_an_error() {
        let sandbox = migrate_sandbox();
        sandbox.enable_git();

        // create a new file at sandbox path to simulate a dirty repository
        sandbox.create_file("new_file", "new_file");

        let assert = sandbox.run_moon(|cmd| {
            cmd.args(["migrate", "from-package-json", "common"]);
        });

        assert
            .failure()
            .code(1)
            .stdout("")
            .stderr(contains("Commit or stash"));
    }

    #[test]
    fn converts_scripts() {
        let sandbox = migrate_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.args([
                "migrate",
                "--skipTouchedFilesCheck",
                "from-package-json",
                "common",
            ]);
        });

        assert_snapshot!(fs::read_to_string(
            sandbox.path().join("package-json/common/package.json")
        )
        .unwrap());

        assert_snapshot!(
            fs::read_to_string(sandbox.path().join("package-json/common/moon.yml")).unwrap()
        );

        assert.success();
    }

    #[test]
    fn links_depends_on() {
        let sandbox = migrate_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.args([
                "migrate",
                "--skipTouchedFilesCheck",
                "from-package-json",
                "deps",
            ]);
        });

        assert_snapshot!(
            fs::read_to_string(sandbox.path().join("package-json/deps/package.json")).unwrap()
        );

        assert_snapshot!(
            fs::read_to_string(sandbox.path().join("package-json/deps/moon.yml")).unwrap()
        );

        assert.success();
    }
}

mod from_turborepo {
    use super::*;
    use moon_cli::commands::migrate::{TurboJson, TurboTask};
    use rustc_hash::FxHashMap;

    #[test]
    fn errors_if_no_config() {
        let sandbox = migrate_sandbox();
        sandbox.enable_git();

        let assert = sandbox.run_moon(|cmd| {
            cmd.args(["migrate", "from-turborepo"]);
        });

        assert
            .failure()
            .code(1)
            .stdout("")
            .stderr(contains("No turbo.json was found in the workspace root."));
    }

    #[test]
    fn converts_globals() {
        let sandbox = migrate_sandbox();
        sandbox.enable_git();

        sandbox.create_file(
            "turbo.json",
            serde_json::to_string_pretty(&TurboJson {
                global_dependencies: Some(string_vec!["package.json", "*.json"]),
                global_env: Some(string_vec!["FOO", "BAR"]),
                ..TurboJson::default()
            })
            .unwrap(),
        );

        let assert = sandbox.run_moon(|cmd| {
            cmd.args(["migrate", "from-turborepo", "--skipTouchedFilesCheck"]);
        });

        assert.success();

        let config = WorkspaceConfig::load(sandbox.path().join(".moon/workspace.yml")).unwrap();

        assert_eq!(
            config.runner.implicit_inputs,
            string_vec![
                "package.json",
                "/.moon/tasks.yml",
                "/.moon/toolchain.yml",
                "/.moon/workspace.yml",
                "package.json",
                "*.json",
                "$FOO",
                "$BAR"
            ]
        );
    }

    #[test]
    fn converts_global_tasks() {
        let sandbox = migrate_sandbox();
        sandbox.enable_git();

        sandbox.create_file(
            "turbo.json",
            serde_json::to_string_pretty(&TurboJson {
                pipeline: FxHashMap::from_iter([
                    (
                        "build".into(),
                        TurboTask {
                            depends_on: Some(string_vec!["^build"]),
                            outputs: Some(string_vec!["build/**"]),
                            ..TurboTask::default()
                        },
                    ),
                    (
                        "lint".into(),
                        TurboTask {
                            cache: Some(false),
                            env: Some(string_vec!["NODE_ENV"]),
                            inputs: Some(string_vec!["src/**/*"]),
                            ..TurboTask::default()
                        },
                    ),
                ]),
                ..TurboJson::default()
            })
            .unwrap(),
        );

        let assert = sandbox.run_moon(|cmd| {
            cmd.args(["migrate", "from-turborepo", "--skipTouchedFilesCheck"]);
        });

        assert.success();

        assert_snapshot!(fs::read_to_string(sandbox.path().join(".moon/tasks.yml")).unwrap());
    }

    #[test]
    fn converts_project_tasks() {
        let sandbox = migrate_sandbox();
        sandbox.enable_git();

        sandbox.create_file(
            "turbo.json",
            serde_json::to_string_pretty(&TurboJson {
                pipeline: FxHashMap::from_iter([
                    (
                        // via package.json name
                        "turborepo-app#build".into(),
                        TurboTask {
                            depends_on: Some(string_vec!["^build"]),
                            outputs: Some(string_vec!["build/**"]),
                            ..TurboTask::default()
                        },
                    ),
                    (
                        // via project id
                        "library#lint".into(),
                        TurboTask {
                            cache: Some(false),
                            env: Some(string_vec!["NODE_ENV"]),
                            inputs: Some(string_vec!["src/**/*"]),
                            ..TurboTask::default()
                        },
                    ),
                ]),
                ..TurboJson::default()
            })
            .unwrap(),
        );

        let assert = sandbox.run_moon(|cmd| {
            cmd.args(["migrate", "from-turborepo", "--skipTouchedFilesCheck"]);
        });

        assert.success();

        assert_snapshot!(
            fs::read_to_string(sandbox.path().join("turborepo/app/moon.yml")).unwrap()
        );

        assert_snapshot!(
            fs::read_to_string(sandbox.path().join("turborepo/library/moon.yml")).unwrap()
        );
    }

    #[test]
    fn ignores_root_tasks() {
        let sandbox = migrate_sandbox();
        sandbox.enable_git();

        sandbox.create_file(
            "turbo.json",
            serde_json::to_string_pretty(&TurboJson {
                pipeline: FxHashMap::from_iter([(
                    "//#build".into(),
                    TurboTask {
                        depends_on: Some(string_vec!["^build"]),
                        outputs: Some(string_vec!["build/**"]),
                        ..TurboTask::default()
                    },
                )]),
                ..TurboJson::default()
            })
            .unwrap(),
        );

        let assert = sandbox.run_moon(|cmd| {
            cmd.args(["migrate", "from-turborepo", "--skipTouchedFilesCheck"]);
        });

        assert
            .success()
            .stderr(contains("Unable to migrate root-level `//#` tasks."));
    }
}
