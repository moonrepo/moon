mod utils;

use httpmock::prelude::*;
use moon_common::consts::CONFIG_TASKS_FILENAME;
use moon_common::Id;
use moon_config::{
    FilePath, GlobPath, InheritedTasksConfig, InheritedTasksManager, InputPath, LanguageType,
    PlatformType, PortablePath, ProjectType, TaskCommandArgs, TaskConfig, TaskOptionsConfig,
};
use moon_target::Target;
use rustc_hash::FxHashMap;
use starbase_sandbox::{create_empty_sandbox, create_sandbox};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use utils::*;

const FILENAME: &str = ".moon/tasks.yml";

mod tasks_config {
    use super::*;

    mod extends {
        use super::*;

        const SHARED_TASKS: &str = r"
fileGroups:
  sources:
    - src/**/*
  tests:
    - tests/**/*

tasks:
  onlyCommand:
    command: a
  stringArgs:
    command: b
    args: string args
  arrayArgs:
    command: c
    args:
      - array
      - args
  inputs:
    command: d
    inputs:
      - src/**/*
  options:
    command: e
    options:
      runInCI: false
";

        fn create_merged_tasks() -> BTreeMap<Id, TaskConfig> {
            BTreeMap::from([
                (
                    "onlyCommand".into(),
                    TaskConfig {
                        command: TaskCommandArgs::String("a".to_owned()),
                        ..TaskConfig::default()
                    },
                ),
                (
                    "stringArgs".into(),
                    TaskConfig {
                        command: TaskCommandArgs::String("b".to_owned()),
                        args: TaskCommandArgs::String("string args".to_owned()),
                        ..TaskConfig::default()
                    },
                ),
                (
                    "arrayArgs".into(),
                    TaskConfig {
                        command: TaskCommandArgs::String("c".to_owned()),
                        args: TaskCommandArgs::Sequence(vec!["array".into(), "args".into()]),
                        ..TaskConfig::default()
                    },
                ),
                (
                    "inputs".into(),
                    TaskConfig {
                        command: TaskCommandArgs::String("d".to_owned()),
                        inputs: Some(vec!["src/**/*".into()]),
                        ..TaskConfig::default()
                    },
                ),
                (
                    "options".into(),
                    TaskConfig {
                        command: TaskCommandArgs::String("e".to_owned()),
                        options: TaskOptionsConfig {
                            run_in_ci: Some(false),
                            ..TaskOptionsConfig::default()
                        },
                        ..TaskConfig::default()
                    },
                ),
            ])
        }

        #[test]
        fn recursive_merges() {
            let sandbox = create_sandbox("extends/tasks");
            let config = test_config(sandbox.path().join("global-2.yml"), |path| {
                InheritedTasksConfig::load(path)
            });

            assert_eq!(
                config.file_groups,
                FxHashMap::from_iter([
                    (
                        "tests".into(),
                        vec![PortablePath::ProjectGlob(GlobPath("tests/**/*".into()))]
                    ),
                    (
                        "sources".into(),
                        vec![PortablePath::ProjectGlob(GlobPath("sources/**/*".into()))]
                    ),
                ])
            );

            assert_eq!(
                *config.tasks.get("lint").unwrap(),
                TaskConfig {
                    command: TaskCommandArgs::String("eslint".to_owned()),
                    ..TaskConfig::default()
                },
            );

            assert_eq!(
                *config.tasks.get("format").unwrap(),
                TaskConfig {
                    command: TaskCommandArgs::String("prettier".to_owned()),
                    ..TaskConfig::default()
                },
            );

            assert_eq!(
                *config.tasks.get("test").unwrap(),
                TaskConfig {
                    command: TaskCommandArgs::String("noop".to_owned()),
                    inputs: None,
                    ..TaskConfig::default()
                },
            );
        }

        #[test]
        fn loads_from_file() {
            let sandbox = create_empty_sandbox();

            sandbox.create_file("shared/tasks.yml", SHARED_TASKS);

            sandbox.create_file(
                "tasks.yml",
                r"
extends: ./shared/tasks.yml

fileGroups:
  sources:
    - sources/**/*
  configs:
    - '/*.js'
",
            );

            let config = test_config(sandbox.path().join("tasks.yml"), |path| {
                InheritedTasksConfig::load(path)
            });

            assert_eq!(
                config.file_groups,
                FxHashMap::from_iter([
                    (
                        "tests".into(),
                        vec![PortablePath::ProjectGlob(GlobPath("tests/**/*".into()))]
                    ),
                    (
                        "sources".into(),
                        vec![PortablePath::ProjectGlob(GlobPath("sources/**/*".into()))]
                    ),
                    (
                        "configs".into(),
                        vec![PortablePath::WorkspaceGlob(GlobPath("*.js".into()))]
                    ),
                ])
            );

            assert_eq!(config.tasks, create_merged_tasks());
        }

        #[test]
        fn loads_from_url() {
            let sandbox = create_empty_sandbox();
            let server = MockServer::start();

            server.mock(|when, then| {
                when.method(GET).path("/config.yml");
                then.status(200).body(SHARED_TASKS);
            });

            let url = server.url("/config.yml");

            dbg!(&url);

            sandbox.create_file(
                "tasks.yml",
                format!(
                    r"
extends: '{url}'

fileGroups:
  sources:
    - sources/**/*
  configs:
    - '/*.js'
"
                ),
            );

            let config = test_config(sandbox.path().join("tasks.yml"), |path| {
                InheritedTasksConfig::load(path)
            });

            assert_eq!(
                config.file_groups,
                FxHashMap::from_iter([
                    (
                        "tests".into(),
                        vec![PortablePath::ProjectGlob(GlobPath("tests/**/*".into()))]
                    ),
                    (
                        "sources".into(),
                        vec![PortablePath::ProjectGlob(GlobPath("sources/**/*".into()))]
                    ),
                    (
                        "configs".into(),
                        vec![PortablePath::WorkspaceGlob(GlobPath("*.js".into()))]
                    ),
                ])
            );

            assert_eq!(config.tasks, create_merged_tasks());
        }
    }

    mod file_groups {
        use super::*;

        #[test]
        fn groups_into_correct_enums() {
            let config = test_load_config(
                FILENAME,
                r"
fileGroups:
  files:
    - /ws/relative
    - proj/relative
  globs:
    - /ws/**/*
    - /!ws/**/*
    - proj/**/*
    - '!proj/**/*'
",
                |path| InheritedTasksConfig::load(path.join(FILENAME)),
            );

            assert_eq!(
                config.file_groups,
                FxHashMap::from_iter([
                    (
                        "files".into(),
                        vec![
                            PortablePath::WorkspaceFile(FilePath("ws/relative".into())),
                            PortablePath::ProjectFile(FilePath("proj/relative".into()))
                        ]
                    ),
                    (
                        "globs".into(),
                        vec![
                            PortablePath::WorkspaceGlob(GlobPath("ws/**/*".into())),
                            PortablePath::WorkspaceGlob(GlobPath("!ws/**/*".into())),
                            PortablePath::ProjectGlob(GlobPath("proj/**/*".into())),
                            PortablePath::ProjectGlob(GlobPath("!proj/**/*".into())),
                        ]
                    ),
                ])
            );
        }
    }

    mod implicit_deps {
        use super::*;

        #[test]
        fn supports_targets() {
            let config = test_load_config(
                FILENAME,
                r"
implicitDeps:
  - task
  - project:task
  - ^:task
  - ~:task
",
                |path| InheritedTasksConfig::load(path.join(FILENAME)),
            );

            assert_eq!(
                config.implicit_deps,
                vec![
                    Target::parse("task").unwrap(),
                    Target::parse("project:task").unwrap(),
                    Target::parse("^:task").unwrap(),
                    Target::parse("~:task").unwrap()
                ]
            );
        }

        #[test]
        #[should_panic(expected = "Invalid target ~:bad target")]
        fn errors_on_invalid_format() {
            test_load_config(FILENAME, "implicitDeps: ['bad target']", |path| {
                InheritedTasksConfig::load(path.join(FILENAME))
            });
        }

        #[test]
        #[should_panic(expected = "target scope not supported as a task dependency")]
        fn errors_on_all_scope() {
            test_load_config(FILENAME, "implicitDeps: [':task']", |path| {
                InheritedTasksConfig::load(path.join(FILENAME))
            });
        }

        #[test]
        #[should_panic(expected = "target scope not supported as a task dependency")]
        fn errors_on_tag_scope() {
            test_load_config(FILENAME, "implicitDeps: ['#tag:task']", |path| {
                InheritedTasksConfig::load(path.join(FILENAME))
            });
        }
    }

    mod implicit_inputs {
        use super::*;

        #[test]
        fn supports_path_patterns() {
            let config = test_load_config(
                FILENAME,
                r"
implicitInputs:
  - /ws/path
  - '/ws/glob/**/*'
  - '/!ws/glob/**/*'
  - proj/path
  - 'proj/glob/{a,b,c}'
  - '!proj/glob/{a,b,c}'
",
                |path| InheritedTasksConfig::load(path.join(FILENAME)),
            );

            assert_eq!(
                config.implicit_inputs,
                vec![
                    InputPath::workspace_file("ws/path"),
                    InputPath::workspace_glob("ws/glob/**/*"),
                    InputPath::workspace_glob("!ws/glob/**/*"),
                    InputPath::project_file("proj/path"),
                    InputPath::project_glob("proj/glob/{a,b,c}"),
                    InputPath::project_glob("!proj/glob/{a,b,c}"),
                ]
            );
        }

        #[test]
        fn supports_env_vars() {
            let config = test_load_config(
                FILENAME,
                r"
implicitInputs:
  - $FOO_BAR
  - file/path
",
                |path| InheritedTasksConfig::load(path.join(FILENAME)),
            );

            assert_eq!(
                config.implicit_inputs,
                vec![
                    InputPath::env_var("FOO_BAR"),
                    InputPath::project_file("file/path"),
                ]
            );
        }
    }
}

mod task_manager {
    use super::*;

    fn load_tasks_into_manager(
        workspace_root: &Path,
        tasks_path: PathBuf,
    ) -> InheritedTasksManager {
        let mut manager = InheritedTasksManager::default();

        manager.add_config(
            &tasks_path,
            InheritedTasksConfig::load_partial(workspace_root, &tasks_path).unwrap(),
        );

        let tasks_dir = tasks_path.parent().unwrap().join("tasks");

        if !tasks_dir.exists() {
            return manager;
        }

        for file in std::fs::read_dir(tasks_dir).unwrap() {
            let file = file.unwrap();

            if file.file_type().unwrap().is_file() {
                manager.add_config(
                    &file.path(),
                    InheritedTasksConfig::load_partial(workspace_root, &file.path()).unwrap(),
                );
            }
        }

        manager
    }

    fn stub_task(command: &str, platform: PlatformType) -> TaskConfig {
        let mut global_inputs = vec![];

        if command != "global" {
            global_inputs.push(format!("/.moon/tasks/{command}.yml"));
        }

        TaskConfig {
            command: TaskCommandArgs::String(command.replace("tag-", "")),
            global_inputs,
            platform,
            ..TaskConfig::default()
        }
    }

    #[test]
    fn loads_all_task_configs_into_manager() {
        let sandbox = create_sandbox("inheritance/files");
        let manager =
            load_tasks_into_manager(sandbox.path(), sandbox.path().join(CONFIG_TASKS_FILENAME));

        let mut keys = manager.configs.keys().collect::<Vec<_>>();
        keys.sort();

        assert_eq!(
            keys,
            vec![
                "*",
                "deno",
                "javascript",
                "javascript-library",
                "javascript-tool",
                "kotlin",
                "node",
                "node-application",
                "node-library",
                "rust",
                "tag-camelCase",
                "tag-dot.case",
                "tag-kebab-case",
                "tag-normal",
                "typescript",
            ]
        );
    }

    mod lookup_order {
        use super::*;

        #[test]
        fn includes_js() {
            let manager = InheritedTasksManager::default();

            assert_eq!(
                manager.get_lookup_order(
                    &PlatformType::Node,
                    &LanguageType::JavaScript,
                    &ProjectType::Application,
                    &[]
                ),
                vec![
                    "*",
                    "node",
                    "javascript",
                    "node-application",
                    "javascript-application"
                ]
            );
        }

        #[test]
        fn includes_ts() {
            let manager = InheritedTasksManager::default();

            assert_eq!(
                manager.get_lookup_order(
                    &PlatformType::Node,
                    &LanguageType::TypeScript,
                    &ProjectType::Library,
                    &[]
                ),
                vec![
                    "*",
                    "node",
                    "typescript",
                    "node-library",
                    "typescript-library"
                ]
            );
        }

        #[test]
        fn supports_langs() {
            let manager = InheritedTasksManager::default();

            assert_eq!(
                manager.get_lookup_order(
                    &PlatformType::Unknown,
                    &LanguageType::Ruby,
                    &ProjectType::Tool,
                    &[]
                ),
                vec!["*", "ruby", "ruby-tool"]
            );

            assert_eq!(
                manager.get_lookup_order(
                    &PlatformType::Unknown,
                    &LanguageType::Rust,
                    &ProjectType::Application,
                    &[]
                ),
                vec!["*", "rust", "rust-application"]
            );
        }

        #[test]
        fn supports_other() {
            let manager = InheritedTasksManager::default();

            assert_eq!(
                manager.get_lookup_order(
                    &PlatformType::Unknown,
                    &LanguageType::Other("kotlin".into()),
                    &ProjectType::Tool,
                    &[]
                ),
                vec!["*", "kotlin", "kotlin-tool"]
            );

            assert_eq!(
                manager.get_lookup_order(
                    &PlatformType::System,
                    &LanguageType::Other("dotnet".into()),
                    &ProjectType::Application,
                    &[]
                ),
                vec!["*", "dotnet", "dotnet-application"]
            );
        }

        #[test]
        fn includes_tags() {
            let manager = InheritedTasksManager::default();

            assert_eq!(
                manager.get_lookup_order(
                    &PlatformType::Unknown,
                    &LanguageType::Rust,
                    &ProjectType::Application,
                    &["cargo".into(), "cli-app".into()]
                ),
                vec!["*", "rust", "rust-application", "tag-cargo", "tag-cli-app"]
            );
        }
    }

    mod config_order {
        use super::*;

        #[test]
        fn creates_js_config() {
            let sandbox = create_sandbox("inheritance/files");
            let manager =
                load_tasks_into_manager(sandbox.path(), sandbox.path().join(CONFIG_TASKS_FILENAME));

            assert_eq!(
                manager
                    .get_inherited_config(
                        &PlatformType::Node,
                        &LanguageType::JavaScript,
                        &ProjectType::Application,
                        &[]
                    )
                    .unwrap()
                    .tasks,
                BTreeMap::from_iter([
                    ("global".into(), stub_task("global", PlatformType::Unknown)),
                    ("node".into(), stub_task("node", PlatformType::Node)),
                    (
                        "node-application".into(),
                        stub_task("node-application", PlatformType::Node)
                    ),
                    (
                        "javascript".into(),
                        stub_task("javascript", PlatformType::Node)
                    ),
                ]),
            );
        }

        #[test]
        fn creates_ts_config() {
            let sandbox = create_sandbox("inheritance/files");
            let manager =
                load_tasks_into_manager(sandbox.path(), sandbox.path().join(CONFIG_TASKS_FILENAME));

            assert_eq!(
                manager
                    .get_inherited_config(
                        &PlatformType::Node,
                        &LanguageType::TypeScript,
                        &ProjectType::Tool,
                        &[]
                    )
                    .unwrap()
                    .tasks,
                BTreeMap::from_iter([
                    ("global".into(), stub_task("global", PlatformType::Unknown)),
                    ("node".into(), stub_task("node", PlatformType::Node)),
                    (
                        "typescript".into(),
                        stub_task("typescript", PlatformType::Node)
                    ),
                ]),
            );
        }

        #[test]
        fn creates_rust_config() {
            let sandbox = create_sandbox("inheritance/files");
            let manager =
                load_tasks_into_manager(sandbox.path(), sandbox.path().join(CONFIG_TASKS_FILENAME));

            assert_eq!(
                manager
                    .get_inherited_config(
                        &PlatformType::System,
                        &LanguageType::Rust,
                        &ProjectType::Library,
                        &[]
                    )
                    .unwrap()
                    .tasks,
                BTreeMap::from_iter([
                    ("global".into(), stub_task("global", PlatformType::Unknown)),
                    ("rust".into(), stub_task("rust", PlatformType::System)),
                ]),
            );
        }

        #[test]
        fn creates_config_with_tags() {
            let sandbox = create_sandbox("inheritance/files");
            let manager =
                load_tasks_into_manager(sandbox.path(), sandbox.path().join(CONFIG_TASKS_FILENAME));

            assert_eq!(
                manager
                    .get_inherited_config(
                        &PlatformType::Node,
                        &LanguageType::TypeScript,
                        &ProjectType::Tool,
                        &["normal".into(), "kebab-case".into()]
                    )
                    .unwrap()
                    .tasks,
                BTreeMap::from_iter([
                    ("global".into(), stub_task("global", PlatformType::Unknown)),
                    ("node".into(), stub_task("node", PlatformType::Node)),
                    (
                        "typescript".into(),
                        stub_task("typescript", PlatformType::Node)
                    ),
                    (
                        "tag".into(),
                        stub_task("tag-kebab-case", PlatformType::Node)
                    ),
                ]),
            );
        }

        #[test]
        fn creates_other_config() {
            let sandbox = create_sandbox("inheritance/files");
            let manager =
                load_tasks_into_manager(sandbox.path(), sandbox.path().join(CONFIG_TASKS_FILENAME));

            assert_eq!(
                manager
                    .get_inherited_config(
                        &PlatformType::System,
                        &LanguageType::Other("kotlin".into()),
                        &ProjectType::Library,
                        &[]
                    )
                    .unwrap()
                    .tasks,
                BTreeMap::from_iter([
                    ("global".into(), stub_task("global", PlatformType::Unknown)),
                    ("kotlin".into(), stub_task("kotlin", PlatformType::System)),
                ]),
            );
        }
    }

    mod config_overrides {
        use super::*;

        #[test]
        fn entirely_overrides_task_of_same_name() {
            let sandbox = create_sandbox("inheritance/override");
            let manager =
                load_tasks_into_manager(sandbox.path(), sandbox.path().join(CONFIG_TASKS_FILENAME));

            let mut task = stub_task("node-library", PlatformType::Node);
            task.inputs = Some(vec!["c".into()]);

            assert_eq!(
                manager
                    .get_inherited_config(
                        &PlatformType::Node,
                        &LanguageType::JavaScript,
                        &ProjectType::Library,
                        &[]
                    )
                    .unwrap()
                    .tasks,
                BTreeMap::from_iter([("command".into(), task)]),
            );
        }

        #[test]
        fn entirely_overrides_task_of_same_name_for_other_lang() {
            let sandbox = create_sandbox("inheritance/override");
            let manager =
                load_tasks_into_manager(sandbox.path(), sandbox.path().join(CONFIG_TASKS_FILENAME));

            let mut task = stub_task("dotnet-application", PlatformType::System);
            task.inputs = Some(vec!["c".into()]);

            assert_eq!(
                manager
                    .get_inherited_config(
                        &PlatformType::System,
                        &LanguageType::Other("dotnet".into()),
                        &ProjectType::Application,
                        &[]
                    )
                    .unwrap()
                    .tasks,
                BTreeMap::from_iter([("command".into(), task)]),
            );
        }
    }
}
