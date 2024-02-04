mod utils;

use httpmock::prelude::*;
use moon_common::Id;
use moon_config::{
    InheritedTasksConfig, InheritedTasksManager, InputPath, LanguageType, PlatformType,
    ProjectType, TaskArgs, TaskConfig, TaskDependency, TaskDependencyConfig, TaskMergeStrategy,
    TaskOptionsConfig,
};
use moon_target::Target;
use rustc_hash::FxHashMap;
use starbase_sandbox::{create_empty_sandbox, create_sandbox};
use std::collections::BTreeMap;
use utils::*;

const FILENAME: &str = "tasks.yml";

mod tasks_config {
    use super::*;

    mod extends {
        use super::*;

        const SHARED_TASKS: &str = r"
inputGroups:
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
                        command: TaskArgs::String("a".to_owned()),
                        ..TaskConfig::default()
                    },
                ),
                (
                    "stringArgs".into(),
                    TaskConfig {
                        command: TaskArgs::String("b".to_owned()),
                        args: TaskArgs::String("string args".to_owned()),
                        ..TaskConfig::default()
                    },
                ),
                (
                    "arrayArgs".into(),
                    TaskConfig {
                        command: TaskArgs::String("c".to_owned()),
                        args: TaskArgs::List(vec!["array".into(), "args".into()]),
                        ..TaskConfig::default()
                    },
                ),
                (
                    "inputs".into(),
                    TaskConfig {
                        command: TaskArgs::String("d".to_owned()),
                        inputs: Some(vec![InputPath::ProjectGlob("src/**/*".into())]),
                        ..TaskConfig::default()
                    },
                ),
                (
                    "options".into(),
                    TaskConfig {
                        command: TaskArgs::String("e".to_owned()),
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
                        vec![InputPath::ProjectGlob("tests/**/*".into())]
                    ),
                    (
                        "sources".into(),
                        vec![InputPath::ProjectGlob("sources/**/*".into())]
                    ),
                ])
            );

            assert_eq!(
                *config.tasks.get("lint").unwrap(),
                TaskConfig {
                    command: TaskArgs::String("eslint".to_owned()),
                    ..TaskConfig::default()
                },
            );

            assert_eq!(
                *config.tasks.get("format").unwrap(),
                TaskConfig {
                    command: TaskArgs::String("prettier".to_owned()),
                    ..TaskConfig::default()
                },
            );

            assert_eq!(
                *config.tasks.get("test").unwrap(),
                TaskConfig {
                    command: TaskArgs::String("noop".to_owned()),
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

inputGroups:
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
                        vec![InputPath::ProjectGlob("tests/**/*".into())]
                    ),
                    (
                        "sources".into(),
                        vec![InputPath::ProjectGlob("sources/**/*".into())]
                    ),
                    (
                        "configs".into(),
                        vec![InputPath::WorkspaceGlob("*.js".into())]
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

            sandbox.create_file(
                "tasks.yml",
                format!(
                    r"
extends: '{url}'

inputGroups:
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
                        vec![InputPath::ProjectGlob("tests/**/*".into())]
                    ),
                    (
                        "sources".into(),
                        vec![InputPath::ProjectGlob("sources/**/*".into())]
                    ),
                    (
                        "configs".into(),
                        vec![InputPath::WorkspaceGlob("*.js".into())]
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
inputGroups:
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
                            InputPath::WorkspaceFile("ws/relative".into()),
                            InputPath::ProjectFile("proj/relative".into())
                        ]
                    ),
                    (
                        "globs".into(),
                        vec![
                            InputPath::WorkspaceGlob("ws/**/*".into()),
                            InputPath::WorkspaceGlob("!ws/**/*".into()),
                            InputPath::ProjectGlob("proj/**/*".into()),
                            InputPath::ProjectGlob("!proj/**/*".into()),
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
                    TaskDependency::Target(Target::parse("task").unwrap()),
                    TaskDependency::Target(Target::parse("project:task").unwrap()),
                    TaskDependency::Target(Target::parse("^:task").unwrap()),
                    TaskDependency::Target(Target::parse("~:task").unwrap()),
                ]
            );
        }

        #[test]
        fn supports_objects() {
            let config = test_load_config(
                FILENAME,
                r"
implicitDeps:
  - target: task
  - args: a b c
    target: project:task
  - env:
      FOO: abc
    target: ^:task
  - args:
      - a
      - b
      - c
    env:
      FOO: abc
    target: ~:task
",
                |path| InheritedTasksConfig::load(path.join(FILENAME)),
            );

            assert_eq!(
                config.implicit_deps,
                vec![
                    TaskDependency::Config(TaskDependencyConfig::new(
                        Target::parse("task").unwrap()
                    )),
                    TaskDependency::Config(TaskDependencyConfig {
                        args: TaskArgs::String("a b c".into()),
                        target: Target::parse("project:task").unwrap(),
                        ..TaskDependencyConfig::default()
                    }),
                    TaskDependency::Config(TaskDependencyConfig {
                        env: FxHashMap::from_iter([("FOO".into(), "abc".into())]),
                        target: Target::parse("^:task").unwrap(),
                        ..TaskDependencyConfig::default()
                    }),
                    TaskDependency::Config(TaskDependencyConfig {
                        args: TaskArgs::List(vec!["a".into(), "b".into(), "c".into()]),
                        env: FxHashMap::from_iter([("FOO".into(), "abc".into())]),
                        target: Target::parse("~:task").unwrap(),
                        optional: None,
                    }),
                ]
            );
        }

        #[test]
        #[should_panic(expected = "expected a valid target or dependency config object")]
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
        #[should_panic(expected = "a target field is required")]
        fn errors_if_using_object_with_no_target() {
            test_load_config(
                FILENAME,
                r"
implicitDeps:
  - args: a b c
",
                |path| InheritedTasksConfig::load(path.join(FILENAME)),
            );
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
                    InputPath::WorkspaceFile("ws/path".into()),
                    InputPath::WorkspaceGlob("ws/glob/**/*".into()),
                    InputPath::WorkspaceGlob("!ws/glob/**/*".into()),
                    InputPath::ProjectFile("proj/path".into()),
                    InputPath::ProjectGlob("proj/glob/{a,b,c}".into()),
                    InputPath::ProjectGlob("!proj/glob/{a,b,c}".into()),
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
                    InputPath::EnvVar("FOO_BAR".into()),
                    InputPath::ProjectFile("file/path".into()),
                ]
            );
        }
    }
}

mod task_manager {
    use super::*;

    fn stub_task(command: &str, platform: PlatformType) -> TaskConfig {
        let mut global_inputs = vec![];

        if command != "global" {
            // No .moon prefix since the fixture is contrived
            global_inputs.push(InputPath::WorkspaceFile(format!("tasks/{command}.yml")));
        }

        TaskConfig {
            command: TaskArgs::String(command.replace("tag-", "")),
            global_inputs,
            platform,
            ..TaskConfig::default()
        }
    }

    #[test]
    fn loads_all_task_configs_into_manager() {
        let sandbox = create_sandbox("inheritance/files");
        let manager = InheritedTasksManager::load(sandbox.path(), sandbox.path()).unwrap();

        let mut keys = manager.configs.keys().collect::<Vec<_>>();
        keys.sort();

        assert_eq!(
            keys,
            vec![
                "*",
                "bun",
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

    #[test]
    fn can_nest_configs_in_folders() {
        let sandbox = create_sandbox("inheritance/nested");
        let manager = InheritedTasksManager::load(sandbox.path(), sandbox.path()).unwrap();

        let mut keys = manager.configs.keys().collect::<Vec<_>>();
        keys.sort();

        assert_eq!(
            keys,
            vec!["*", "dotnet", "dotnet-application", "node", "node-library",]
        );

        let mut inputs = manager
            .configs
            .values()
            .map(|c| c.input.to_string_lossy().replace('\\', "/"))
            .collect::<Vec<_>>();
        inputs.sort();

        assert_eq!(
            inputs,
            vec![
                "tasks.yml",
                "tasks/dotnet/dotnet-application.yml",
                "tasks/dotnet/dotnet.yml",
                "tasks/node/node-library.yml",
                "tasks/node/node.yml"
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
            let manager = InheritedTasksManager::load(sandbox.path(), sandbox.path()).unwrap();

            let config = manager
                .get_inherited_config(
                    &PlatformType::Node,
                    &LanguageType::JavaScript,
                    &ProjectType::Application,
                    &[],
                )
                .unwrap();

            assert_eq!(
                config.config.tasks,
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

            assert_eq!(
                config.layers.keys().collect::<Vec<_>>(),
                vec![
                    "tasks.yml",
                    "tasks/javascript.yml",
                    "tasks/node-application.yml",
                    "tasks/node.yml",
                ]
            );
        }

        #[test]
        fn creates_js_config_via_bun() {
            use starbase_sandbox::pretty_assertions::assert_eq;

            let sandbox = create_sandbox("inheritance/files");
            let manager = InheritedTasksManager::load(sandbox.path(), sandbox.path()).unwrap();

            let config = manager
                .get_inherited_config(
                    &PlatformType::Bun,
                    &LanguageType::JavaScript,
                    &ProjectType::Application,
                    &[],
                )
                .unwrap();

            assert_eq!(
                config.config.tasks,
                BTreeMap::from_iter([
                    ("global".into(), stub_task("global", PlatformType::Unknown)),
                    ("bun".into(), stub_task("bun", PlatformType::Bun)),
                    (
                        "javascript".into(),
                        stub_task("javascript", PlatformType::Bun)
                    ),
                ]),
            );

            assert_eq!(
                config.layers.keys().collect::<Vec<_>>(),
                vec!["tasks.yml", "tasks/bun.yml", "tasks/javascript.yml",]
            );
        }

        #[test]
        fn creates_ts_config() {
            let sandbox = create_sandbox("inheritance/files");
            let manager = InheritedTasksManager::load(sandbox.path(), sandbox.path()).unwrap();

            let config = manager
                .get_inherited_config(
                    &PlatformType::Node,
                    &LanguageType::TypeScript,
                    &ProjectType::Tool,
                    &[],
                )
                .unwrap();

            assert_eq!(
                config.config.tasks,
                BTreeMap::from_iter([
                    ("global".into(), stub_task("global", PlatformType::Unknown)),
                    ("node".into(), stub_task("node", PlatformType::Node)),
                    (
                        "typescript".into(),
                        stub_task("typescript", PlatformType::Node)
                    ),
                ]),
            );

            assert_eq!(
                config.layers.keys().collect::<Vec<_>>(),
                vec!["tasks.yml", "tasks/node.yml", "tasks/typescript.yml",]
            );
        }

        #[test]
        fn creates_rust_config() {
            let sandbox = create_sandbox("inheritance/files");
            let manager = InheritedTasksManager::load(sandbox.path(), sandbox.path()).unwrap();

            let config = manager
                .get_inherited_config(
                    &PlatformType::System,
                    &LanguageType::Rust,
                    &ProjectType::Library,
                    &[],
                )
                .unwrap();

            assert_eq!(
                config.config.tasks,
                BTreeMap::from_iter([
                    ("global".into(), stub_task("global", PlatformType::Unknown)),
                    ("rust".into(), stub_task("rust", PlatformType::System)),
                ]),
            );

            assert_eq!(
                config.layers.keys().collect::<Vec<_>>(),
                vec!["tasks.yml", "tasks/rust.yml",]
            );
        }

        #[test]
        fn creates_config_with_tags() {
            let sandbox = create_sandbox("inheritance/files");
            let manager = InheritedTasksManager::load(sandbox.path(), sandbox.path()).unwrap();

            let config = manager
                .get_inherited_config(
                    &PlatformType::Node,
                    &LanguageType::TypeScript,
                    &ProjectType::Tool,
                    &["normal".into(), "kebab-case".into()],
                )
                .unwrap();

            assert_eq!(
                config.config.tasks,
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

            assert_eq!(
                config.layers.keys().collect::<Vec<_>>(),
                vec![
                    "tasks.yml",
                    "tasks/node.yml",
                    "tasks/tag-kebab-case.yml",
                    "tasks/tag-normal.yml",
                    "tasks/typescript.yml",
                ]
            );
        }

        #[test]
        fn creates_other_config() {
            let sandbox = create_sandbox("inheritance/files");
            let manager = InheritedTasksManager::load(sandbox.path(), sandbox.path()).unwrap();

            let config = manager
                .get_inherited_config(
                    &PlatformType::System,
                    &LanguageType::Other("kotlin".into()),
                    &ProjectType::Library,
                    &[],
                )
                .unwrap();

            assert_eq!(
                config.config.tasks,
                BTreeMap::from_iter([
                    ("global".into(), stub_task("global", PlatformType::Unknown)),
                    ("kotlin".into(), stub_task("kotlin", PlatformType::System)),
                ]),
            );

            assert_eq!(
                config.layers.keys().collect::<Vec<_>>(),
                vec!["tasks.yml", "tasks/kotlin.yml",]
            );
        }
    }

    mod config_overrides {
        use super::*;

        #[test]
        fn entirely_overrides_task_of_same_name() {
            let sandbox = create_sandbox("inheritance/override");
            let manager = InheritedTasksManager::load(sandbox.path(), sandbox.path()).unwrap();

            let mut task = stub_task("node-library", PlatformType::Node);
            task.inputs = Some(vec![InputPath::ProjectFile("c".into())]);

            let config = manager
                .get_inherited_config(
                    &PlatformType::Node,
                    &LanguageType::JavaScript,
                    &ProjectType::Library,
                    &[],
                )
                .unwrap();

            assert_eq!(
                config.config.tasks,
                BTreeMap::from_iter([("command".into(), task)]),
            );
        }

        #[test]
        fn entirely_overrides_task_of_same_name_for_other_lang() {
            let sandbox = create_sandbox("inheritance/override");
            let manager = InheritedTasksManager::load(sandbox.path(), sandbox.path()).unwrap();

            let mut task = stub_task("dotnet-application", PlatformType::System);
            task.inputs = Some(vec![InputPath::ProjectFile("c".into())]);

            let config = manager
                .get_inherited_config(
                    &PlatformType::System,
                    &LanguageType::Other("dotnet".into()),
                    &ProjectType::Application,
                    &[],
                )
                .unwrap();

            assert_eq!(
                config.config.tasks,
                BTreeMap::from_iter([("command".into(), task)]),
            );
        }
    }

    mod task_options {
        use super::*;

        #[test]
        fn uses_defaults() {
            let sandbox = create_sandbox("inheritance/options");
            let manager = InheritedTasksManager::load(sandbox.path(), sandbox.path()).unwrap();

            let config = manager
                .get_inherited_config(
                    &PlatformType::Rust,
                    &LanguageType::Rust,
                    &ProjectType::Application,
                    &[],
                )
                .unwrap();

            let options = config.config.task_options.unwrap();

            assert_eq!(options.cache, None);
            assert_eq!(options.shell, None);
            assert_eq!(options.merge_args, Some(TaskMergeStrategy::Replace));
        }

        #[test]
        fn merges_all_options() {
            let sandbox = create_sandbox("inheritance/options");
            let manager = InheritedTasksManager::load(sandbox.path(), sandbox.path()).unwrap();

            let config = manager
                .get_inherited_config(
                    &PlatformType::Node,
                    &LanguageType::JavaScript,
                    &ProjectType::Library,
                    &[],
                )
                .unwrap();

            let options = config.config.task_options.unwrap();

            assert_eq!(options.cache, Some(false));
            assert_eq!(options.shell, Some(true));
            assert_eq!(options.merge_args, Some(TaskMergeStrategy::Prepend));
        }
    }
}
