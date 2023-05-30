mod utils;

use httpmock::prelude::*;
use moon_common::Id;
use moon_config::{
    FilePath, GlobPath, InheritedTasksConfig, PortablePath, TaskCommandArgs, TaskConfig,
    TaskOptionsConfig,
};
use moon_target::Target;
use rustc_hash::FxHashMap;
use starbase_sandbox::{create_empty_sandbox, create_sandbox};
use std::collections::BTreeMap;
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
            let sandbox = create_sandbox("tasks-extends");
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
                    "/ws/path".to_owned(),
                    "/ws/glob/**/*".to_owned(),
                    "/!ws/glob/**/*".to_owned(),
                    "proj/path".to_owned(),
                    "proj/glob/{a,b,c}".to_owned(),
                    "!proj/glob/{a,b,c}".to_owned(),
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
                vec!["$FOO_BAR".to_owned(), "file/path".to_owned(),]
            );
        }
    }
}
