mod utils;

use httpmock::prelude::*;
use indexmap::IndexMap;
use moon_common::Id;
use moon_config::*;
use moon_target::Target;
use rustc_hash::FxHashMap;
use starbase_sandbox::{create_empty_sandbox, create_sandbox};
use std::collections::BTreeMap;
use std::path::Path;
use utils::*;

const FILENAME: &str = "tasks/all.yml";

fn load_config_from_file(path: &Path) -> miette::Result<InheritedTasksConfig> {
    ConfigLoader::new(path.join(".moon")).load_tasks_config_from_path(path.parent().unwrap(), path)
}

fn load_manager_from_root(root: &Path, moon_dir: &Path) -> miette::Result<InheritedTasksManager> {
    ConfigLoader::new(moon_dir).load_tasks_manager_from(root, moon_dir)
}

fn create_inherit_for<'a>(
    toolchains: &'a [Id],
    stack: &'a StackType,
    layer: &'a LayerType,
    tags: &'a [Id],
) -> InheritFor<'a> {
    InheritFor {
        language: None,
        layer: Some(layer),
        root: None,
        stack: Some(stack),
        tags: Some(tags),
        toolchains: Some(toolchains),
    }
}

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
                    Id::raw("onlyCommand"),
                    TaskConfig {
                        command: TaskArgs::String("a".to_owned()),
                        ..TaskConfig::default()
                    },
                ),
                (
                    Id::raw("stringArgs"),
                    TaskConfig {
                        command: TaskArgs::String("b".to_owned()),
                        args: TaskArgs::String("string args".to_owned()),
                        ..TaskConfig::default()
                    },
                ),
                (
                    Id::raw("arrayArgs"),
                    TaskConfig {
                        command: TaskArgs::String("c".to_owned()),
                        args: TaskArgs::List(vec!["array".into(), "args".into()]),
                        ..TaskConfig::default()
                    },
                ),
                (
                    Id::raw("inputs"),
                    TaskConfig {
                        command: TaskArgs::String("d".to_owned()),
                        inputs: Some(vec![Input::Glob(stub_glob_input("src/**/*"))]),
                        ..TaskConfig::default()
                    },
                ),
                (
                    Id::raw("options"),
                    TaskConfig {
                        command: TaskArgs::String("e".to_owned()),
                        options: TaskOptionsConfig {
                            run_in_ci: Some(TaskOptionRunInCI::Enabled(false)),
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
                load_config_from_file(path)
            });

            assert_eq!(
                config.file_groups,
                FxHashMap::from_iter([
                    (
                        Id::raw("tests"),
                        vec![Input::Glob(stub_glob_input("tests/**/*"))]
                    ),
                    (
                        Id::raw("sources"),
                        vec![Input::Glob(stub_glob_input("sources/**/*"))]
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
                    inputs: Some(vec![Input::File(stub_file_input("tests"))]),
                    ..TaskConfig::default()
                },
            );
        }

        #[test]
        fn loads_from_file() {
            let sandbox = create_empty_sandbox();

            sandbox.create_file("shared/tasks/all.yml", SHARED_TASKS);

            sandbox.create_file(
                "tasks/all.yml",
                r"
extends: ../shared/tasks/all.yml

fileGroups:
  sources:
    - sources/**/*
  configs:
    - '/*.js'
",
            );

            let config = test_config(sandbox.path().join("tasks/all.yml"), |path| {
                load_config_from_file(path)
            });

            assert_eq!(
                config.file_groups,
                FxHashMap::from_iter([
                    (
                        Id::raw("tests"),
                        vec![Input::Glob(stub_glob_input("tests/**/*"))]
                    ),
                    (
                        Id::raw("sources"),
                        vec![Input::Glob(stub_glob_input("sources/**/*"))]
                    ),
                    (
                        Id::raw("configs"),
                        vec![Input::Glob(stub_glob_input("/*.js"))]
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
                "tasks/all.yml",
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

            let config = test_config(sandbox.path().join("tasks/all.yml"), |path| {
                load_config_from_file(path)
            });

            assert_eq!(
                config.file_groups,
                FxHashMap::from_iter([
                    (
                        Id::raw("tests"),
                        vec![Input::Glob(stub_glob_input("tests/**/*"))]
                    ),
                    (
                        Id::raw("sources"),
                        vec![Input::Glob(stub_glob_input("sources/**/*"))]
                    ),
                    (
                        Id::raw("configs"),
                        vec![Input::Glob(stub_glob_input("/*.js"))]
                    ),
                ])
            );

            assert_eq!(config.tasks, create_merged_tasks());
        }

        #[test]
        fn loads_from_url_and_saves_temp_file() {
            let sandbox = create_empty_sandbox();
            let server = MockServer::start();

            server.mock(|when, then| {
                when.method(GET).path("/config.yml");
                then.status(200).body(SHARED_TASKS);
            });

            let temp_dir = sandbox.path().join(".moon/cache/temp");
            let url = server.url("/config.yml");

            sandbox.create_file("tasks/all.yml", format!(r"extends: '{url}'"));

            assert!(!temp_dir.exists());

            test_config(sandbox.path().join("tasks/all.yml"), |path| {
                let config = ConfigLoader::new(sandbox.path().join(".moon"))
                    .load_tasks_config_from_path(sandbox.path(), path)
                    .unwrap();

                Ok(config)
            });

            assert!(temp_dir.exists());
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
                |path| load_config_from_file(&path.join(FILENAME)),
            );

            assert_eq!(
                config.file_groups,
                FxHashMap::from_iter([
                    (
                        Id::raw("files"),
                        vec![
                            Input::File(stub_file_input("/ws/relative")),
                            Input::File(stub_file_input("proj/relative"))
                        ]
                    ),
                    (
                        Id::raw("globs"),
                        vec![
                            Input::Glob(stub_glob_input("/ws/**/*")),
                            Input::Glob(stub_glob_input("!/ws/**/*")),
                            Input::Glob(stub_glob_input("proj/**/*")),
                            Input::Glob(stub_glob_input("!proj/**/*")),
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
                |path| load_config_from_file(&path.join(FILENAME)),
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
  - args: [a, b, c]
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
                |path| load_config_from_file(&path.join(FILENAME)),
            );

            assert_eq!(
                config.implicit_deps,
                vec![
                    TaskDependency::Config(TaskDependencyConfig::new(
                        Target::parse("task").unwrap()
                    )),
                    TaskDependency::Config(TaskDependencyConfig {
                        args: vec!["a".into(), "b".into(), "c".into()],
                        target: Target::parse("project:task").unwrap(),
                        ..TaskDependencyConfig::default()
                    }),
                    TaskDependency::Config(TaskDependencyConfig {
                        env: IndexMap::from_iter([("FOO".into(), Some("abc".to_owned()))]),
                        target: Target::parse("^:task").unwrap(),
                        ..TaskDependencyConfig::default()
                    }),
                    TaskDependency::Config(TaskDependencyConfig {
                        args: vec!["a".into(), "b".into(), "c".into()],
                        env: IndexMap::from_iter([("FOO".into(), Some("abc".to_owned()))]),
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
                load_config_from_file(&path.join(FILENAME))
            });
        }

        #[test]
        #[should_panic(expected = "target scope not supported as a task dependency")]
        fn errors_on_all_scope() {
            test_load_config(FILENAME, "implicitDeps: [':task']", |path| {
                load_config_from_file(&path.join(FILENAME))
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
                |path| load_config_from_file(&path.join(FILENAME)),
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
                |path| load_config_from_file(&path.join(FILENAME)),
            );

            assert_eq!(
                config.implicit_inputs,
                vec![
                    Input::File(stub_file_input("/ws/path")),
                    Input::Glob(stub_glob_input("/ws/glob/**/*")),
                    Input::Glob(stub_glob_input("!/ws/glob/**/*")),
                    Input::File(stub_file_input("proj/path")),
                    Input::Glob(stub_glob_input("proj/glob/{a,b,c}")),
                    Input::Glob(stub_glob_input("!proj/glob/{a,b,c}")),
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
                |path| load_config_from_file(&path.join(FILENAME)),
            );

            assert_eq!(
                config.implicit_inputs,
                vec![
                    Input::EnvVar("FOO_BAR".into()),
                    Input::File(stub_file_input("file/path")),
                ]
            );
        }
    }

    mod inherited_by {
        use super::*;

        #[test]
        fn one_file() {
            let config = test_load_config(
                FILENAME,
                r"
inheritedBy:
  file: config.js
",
                |path| load_config_from_file(&path.join(FILENAME)),
            );

            assert_eq!(
                config.inherited_by.unwrap().files.unwrap(),
                OneOrMany::One(FilePath("config.js".into())),
            );
        }

        #[test]
        fn many_files() {
            let config = test_load_config(
                FILENAME,
                r"
inheritedBy:
  files: [a.json, b.json]
",
                |path| load_config_from_file(&path.join(FILENAME)),
            );

            assert_eq!(
                config.inherited_by.unwrap().files.unwrap(),
                OneOrMany::Many(vec![FilePath("a.json".into()), FilePath("b.json".into())]),
            );
        }

        #[should_panic]
        #[test]
        fn errors_for_glob() {
            test_load_config(
                FILENAME,
                r"
inheritedBy:
  file: config.*
",
                |path| load_config_from_file(&path.join(FILENAME)),
            );
        }

        #[test]
        fn one_language() {
            let config = test_load_config(
                FILENAME,
                r"
inheritedBy:
  language: bash
",
                |path| load_config_from_file(&path.join(FILENAME)),
            );

            assert_eq!(
                config.inherited_by.unwrap().languages.unwrap(),
                OneOrMany::One(LanguageType::Bash),
            );
        }

        #[test]
        fn many_languages() {
            let config = test_load_config(
                FILENAME,
                r"
inheritedBy:
  languages: [bash, batch]
",
                |path| load_config_from_file(&path.join(FILENAME)),
            );

            assert_eq!(
                config.inherited_by.unwrap().languages.unwrap(),
                OneOrMany::Many(vec![LanguageType::Bash, LanguageType::Batch]),
            );
        }

        #[test]
        fn one_layer() {
            let config = test_load_config(
                FILENAME,
                r"
inheritedBy:
  layer: library
",
                |path| load_config_from_file(&path.join(FILENAME)),
            );

            assert_eq!(
                config.inherited_by.unwrap().layers.unwrap(),
                OneOrMany::One(LayerType::Library),
            );
        }

        #[test]
        fn many_layers() {
            let config = test_load_config(
                FILENAME,
                r"
inheritedBy:
  layers: [library, application]
",
                |path| load_config_from_file(&path.join(FILENAME)),
            );

            assert_eq!(
                config.inherited_by.unwrap().layers.unwrap(),
                OneOrMany::Many(vec![LayerType::Library, LayerType::Application]),
            );
        }

        #[test]
        fn one_stack() {
            let config = test_load_config(
                FILENAME,
                r"
inheritedBy:
  stack: frontend
",
                |path| load_config_from_file(&path.join(FILENAME)),
            );

            assert_eq!(
                config.inherited_by.unwrap().stacks.unwrap(),
                OneOrMany::One(StackType::Frontend),
            );
        }

        #[test]
        fn many_stacks() {
            let config = test_load_config(
                FILENAME,
                r"
inheritedBy:
  stacks: [frontend, data]
",
                |path| load_config_from_file(&path.join(FILENAME)),
            );

            assert_eq!(
                config.inherited_by.unwrap().stacks.unwrap(),
                OneOrMany::Many(vec![StackType::Frontend, StackType::Data]),
            );
        }

        #[test]
        fn one_tag() {
            let config = test_load_config(
                FILENAME,
                r"
inheritedBy:
  tag: a
",
                |path| load_config_from_file(&path.join(FILENAME)),
            );

            assert_eq!(
                config.inherited_by.unwrap().tags.unwrap(),
                InheritedConditionConfig::One(Id::raw("a"))
            );
        }

        #[test]
        fn many_tags() {
            let config = test_load_config(
                FILENAME,
                r"
inheritedBy:
  tags: [a, b]
",
                |path| load_config_from_file(&path.join(FILENAME)),
            );

            assert_eq!(
                config.inherited_by.unwrap().tags.unwrap(),
                InheritedConditionConfig::Many(vec![Id::raw("a"), Id::raw("b")])
            );
        }

        #[test]
        fn clause_tags() {
            let config = test_load_config(
                FILENAME,
                r"
inheritedBy:
  tags:
    and: [a, b]
    or: c
    not: d
",
                |path| load_config_from_file(&path.join(FILENAME)),
            );

            assert_eq!(
                config.inherited_by.unwrap().tags.unwrap(),
                InheritedConditionConfig::Clause(InheritedClauseConfig {
                    and: Some(OneOrMany::Many(vec![Id::raw("a"), Id::raw("b")])),
                    or: Some(OneOrMany::One(Id::raw("c"))),
                    not: Some(OneOrMany::One(Id::raw("d")))
                })
            );
        }

        #[test]
        fn one_toolchain() {
            let config = test_load_config(
                FILENAME,
                r"
inheritedBy:
  toolchain: a
",
                |path| load_config_from_file(&path.join(FILENAME)),
            );

            assert_eq!(
                config.inherited_by.unwrap().toolchains.unwrap(),
                InheritedConditionConfig::One(Id::raw("a"))
            );
        }

        #[test]
        fn many_toolchains() {
            let config = test_load_config(
                FILENAME,
                r"
inheritedBy:
  toolchains: [a, b]
",
                |path| load_config_from_file(&path.join(FILENAME)),
            );

            assert_eq!(
                config.inherited_by.unwrap().toolchains.unwrap(),
                InheritedConditionConfig::Many(vec![Id::raw("a"), Id::raw("b")])
            );
        }

        #[test]
        fn clause_toolchains() {
            let config = test_load_config(
                FILENAME,
                r"
inheritedBy:
  toolchains:
    and: [a, b]
    or: c
    not: d
",
                |path| load_config_from_file(&path.join(FILENAME)),
            );

            assert_eq!(
                config.inherited_by.unwrap().toolchains.unwrap(),
                InheritedConditionConfig::Clause(InheritedClauseConfig {
                    and: Some(OneOrMany::Many(vec![Id::raw("a"), Id::raw("b")])),
                    or: Some(OneOrMany::One(Id::raw("c"))),
                    not: Some(OneOrMany::One(Id::raw("d")))
                })
            );
        }

        #[test]
        fn clause_one_and() {
            let config = test_load_config(
                FILENAME,
                r"
inheritedBy:
  toolchains:
    and: a
",
                |path| load_config_from_file(&path.join(FILENAME)),
            );

            assert_eq!(
                config.inherited_by.unwrap().toolchains.unwrap(),
                InheritedConditionConfig::Clause(InheritedClauseConfig {
                    and: Some(OneOrMany::One(Id::raw("a"))),
                    ..Default::default()
                })
            );
        }

        #[test]
        fn clause_many_and() {
            let config = test_load_config(
                FILENAME,
                r"
inheritedBy:
  toolchains:
    and: [a, b]
",
                |path| load_config_from_file(&path.join(FILENAME)),
            );

            assert_eq!(
                config.inherited_by.unwrap().toolchains.unwrap(),
                InheritedConditionConfig::Clause(InheritedClauseConfig {
                    and: Some(OneOrMany::Many(vec![Id::raw("a"), Id::raw("b")])),
                    ..Default::default()
                })
            );
        }

        #[test]
        fn clause_one_or() {
            let config = test_load_config(
                FILENAME,
                r"
inheritedBy:
  toolchains:
    or: a
",
                |path| load_config_from_file(&path.join(FILENAME)),
            );

            assert_eq!(
                config.inherited_by.unwrap().toolchains.unwrap(),
                InheritedConditionConfig::Clause(InheritedClauseConfig {
                    or: Some(OneOrMany::One(Id::raw("a"))),
                    ..Default::default()
                })
            );
        }

        #[test]
        fn clause_many_or() {
            let config = test_load_config(
                FILENAME,
                r"
inheritedBy:
  toolchains:
    or: [a, b]
",
                |path| load_config_from_file(&path.join(FILENAME)),
            );

            assert_eq!(
                config.inherited_by.unwrap().toolchains.unwrap(),
                InheritedConditionConfig::Clause(InheritedClauseConfig {
                    or: Some(OneOrMany::Many(vec![Id::raw("a"), Id::raw("b")])),
                    ..Default::default()
                })
            );
        }

        #[test]
        fn clause_one_not() {
            let config = test_load_config(
                FILENAME,
                r"
inheritedBy:
  toolchains:
    not: a
",
                |path| load_config_from_file(&path.join(FILENAME)),
            );

            assert_eq!(
                config.inherited_by.unwrap().toolchains.unwrap(),
                InheritedConditionConfig::Clause(InheritedClauseConfig {
                    not: Some(OneOrMany::One(Id::raw("a"))),
                    ..Default::default()
                })
            );
        }

        #[test]
        fn clause_many_not() {
            let config = test_load_config(
                FILENAME,
                r"
inheritedBy:
  toolchains:
    not: [a, b]
",
                |path| load_config_from_file(&path.join(FILENAME)),
            );

            assert_eq!(
                config.inherited_by.unwrap().toolchains.unwrap(),
                InheritedConditionConfig::Clause(InheritedClauseConfig {
                    not: Some(OneOrMany::Many(vec![Id::raw("a"), Id::raw("b")])),
                    ..Default::default()
                })
            );
        }

        #[test]
        fn clause_and_or() {
            let config = test_load_config(
                FILENAME,
                r"
inheritedBy:
  toolchains:
    and: [a, b]
    or: c
",
                |path| load_config_from_file(&path.join(FILENAME)),
            );

            assert_eq!(
                config.inherited_by.unwrap().toolchains.unwrap(),
                InheritedConditionConfig::Clause(InheritedClauseConfig {
                    and: Some(OneOrMany::Many(vec![Id::raw("a"), Id::raw("b")])),
                    or: Some(OneOrMany::One(Id::raw("c"))),
                    ..Default::default()
                })
            );
        }

        #[test]
        fn clause_and_not() {
            let config = test_load_config(
                FILENAME,
                r"
inheritedBy:
  toolchains:
    and: [a, b]
    not: [c, d]
",
                |path| load_config_from_file(&path.join(FILENAME)),
            );

            assert_eq!(
                config.inherited_by.unwrap().toolchains.unwrap(),
                InheritedConditionConfig::Clause(InheritedClauseConfig {
                    and: Some(OneOrMany::Many(vec![Id::raw("a"), Id::raw("b")])),
                    not: Some(OneOrMany::Many(vec![Id::raw("c"), Id::raw("d")])),
                    ..Default::default()
                })
            );
        }

        #[test]
        fn clause_or_not() {
            let config = test_load_config(
                FILENAME,
                r"
inheritedBy:
  toolchains:
    or: a
    not: b
",
                |path| load_config_from_file(&path.join(FILENAME)),
            );

            assert_eq!(
                config.inherited_by.unwrap().toolchains.unwrap(),
                InheritedConditionConfig::Clause(InheritedClauseConfig {
                    or: Some(OneOrMany::One(Id::raw("a"))),
                    not: Some(OneOrMany::One(Id::raw("b"))),
                    ..Default::default()
                })
            );
        }
    }
}

mod task_manager {
    use super::*;

    fn get_config_paths(entries: &[InheritedTasksEntry]) -> Vec<String> {
        let mut list = entries
            .iter()
            .map(|entry| entry.input.as_str().to_string())
            .collect::<Vec<_>>();
        list.sort();
        list
    }

    #[test]
    fn loads_all_task_configs_into_manager() {
        let sandbox = create_sandbox("inheritance/files");
        let manager = load_manager_from_root(sandbox.path(), sandbox.path()).unwrap();

        assert_eq!(
            get_config_paths(&manager.configs),
            vec![
                "tasks/all.yml",
                "tasks/bun.yml",
                "tasks/deno.yml",
                "tasks/javascript-library.yml",
                "tasks/javascript-tool.yml",
                "tasks/javascript.yml",
                "tasks/kotlin.yml",
                "tasks/node-application.yml",
                "tasks/node-library.yml",
                "tasks/node.yml",
                "tasks/python.yml",
                "tasks/rust.yml",
                "tasks/tag-camelCase.yml",
                "tasks/tag-dot.case.yml",
                "tasks/tag-kebab-case.yml",
                "tasks/tag-normal.yml",
                "tasks/typescript.yml",
            ]
        );
    }

    #[test]
    fn can_nest_configs_in_folders() {
        let sandbox = create_sandbox("inheritance/nested");
        let manager = load_manager_from_root(sandbox.path(), sandbox.path()).unwrap();

        assert_eq!(
            get_config_paths(&manager.configs),
            vec![
                "tasks/all.yml",
                "tasks/dotnet/dotnet-application.yml",
                "tasks/dotnet/dotnet.yml",
                "tasks/node/node-library.yml",
                "tasks/node/node.yml"
            ]
        );
    }

    mod config_order {
        use super::*;
        use starbase_sandbox::pretty_assertions::assert_eq;

        #[test]
        fn creates_js_config() {
            let sandbox = create_sandbox("inheritance/files");
            let manager = load_manager_from_root(sandbox.path(), sandbox.path()).unwrap();

            let config = manager
                .get_inherited_config(create_inherit_for(
                    &[Id::raw("node"), Id::raw("javascript")],
                    &StackType::Backend,
                    &LayerType::Application,
                    &[],
                ))
                .unwrap();

            assert_eq!(
                config.configs.keys().collect::<Vec<_>>(),
                vec![
                    "tasks/all.yml",
                    "tasks/javascript.yml",
                    "tasks/node.yml",
                    "tasks/node-application.yml",
                ]
            );
        }

        #[test]
        fn creates_js_config_via_bun() {
            let sandbox = create_sandbox("inheritance/files");
            let manager = load_manager_from_root(sandbox.path(), sandbox.path()).unwrap();

            let config = manager
                .get_inherited_config(create_inherit_for(
                    &[Id::raw("bun"), Id::raw("javascript")],
                    &StackType::Backend,
                    &LayerType::Application,
                    &[],
                ))
                .unwrap();

            assert_eq!(
                config.configs.keys().collect::<Vec<_>>(),
                vec!["tasks/all.yml", "tasks/bun.yml", "tasks/javascript.yml"]
            );
        }

        #[test]
        fn creates_ts_config() {
            let sandbox = create_sandbox("inheritance/files");
            let manager = load_manager_from_root(sandbox.path(), sandbox.path()).unwrap();

            let config = manager
                .get_inherited_config(create_inherit_for(
                    &[Id::raw("node"), Id::raw("typescript")],
                    &StackType::Frontend,
                    &LayerType::Tool,
                    &[],
                ))
                .unwrap();

            assert_eq!(
                config.configs.keys().collect::<Vec<_>>(),
                vec!["tasks/all.yml", "tasks/node.yml", "tasks/typescript.yml"]
            );
        }

        #[test]
        fn creates_python_config() {
            let sandbox = create_sandbox("inheritance/files");
            let manager = load_manager_from_root(sandbox.path(), sandbox.path()).unwrap();

            let config = manager
                .get_inherited_config(create_inherit_for(
                    &[Id::raw("python")],
                    &StackType::Frontend,
                    &LayerType::Library,
                    &[],
                ))
                .unwrap();

            assert_eq!(
                config.configs.keys().collect::<Vec<_>>(),
                vec!["tasks/all.yml", "tasks/python.yml"]
            );
        }

        #[test]
        fn creates_rust_config() {
            let sandbox = create_sandbox("inheritance/files");
            let manager = load_manager_from_root(sandbox.path(), sandbox.path()).unwrap();

            let config = manager
                .get_inherited_config(create_inherit_for(
                    &[Id::raw("rust")],
                    &StackType::Frontend,
                    &LayerType::Library,
                    &[],
                ))
                .unwrap();

            assert_eq!(
                config.configs.keys().collect::<Vec<_>>(),
                vec!["tasks/all.yml", "tasks/rust.yml"]
            );
        }

        #[test]
        fn creates_config_with_tags() {
            let sandbox = create_sandbox("inheritance/files");
            let manager = load_manager_from_root(sandbox.path(), sandbox.path()).unwrap();

            let config = manager
                .get_inherited_config(create_inherit_for(
                    &[Id::raw("node"), Id::raw("typescript")],
                    &StackType::Frontend,
                    &LayerType::Tool,
                    &[Id::raw("normal"), Id::raw("kebab-case")],
                ))
                .unwrap();

            assert_eq!(
                config.configs.keys().collect::<Vec<_>>(),
                vec![
                    "tasks/all.yml",
                    "tasks/node.yml",
                    "tasks/typescript.yml",
                    "tasks/tag-kebab-case.yml",
                    "tasks/tag-normal.yml",
                ]
            );
        }

        #[test]
        fn creates_other_config() {
            let sandbox = create_sandbox("inheritance/files");
            let manager = load_manager_from_root(sandbox.path(), sandbox.path()).unwrap();

            let config = manager
                .get_inherited_config(create_inherit_for(
                    &[Id::raw("kotlin"), Id::raw("system")],
                    &StackType::Frontend,
                    &LayerType::Library,
                    &[],
                ))
                .unwrap();

            assert_eq!(
                config.configs.keys().collect::<Vec<_>>(),
                vec!["tasks/all.yml", "tasks/kotlin.yml"]
            );
        }
    }

    #[test]
    fn supports_hcl() {
        load_tasks_config_in_format("hcl");
    }

    #[test]
    fn supports_pkl() {
        load_tasks_config_in_format("pkl");
    }

    #[test]
    fn supports_toml() {
        load_tasks_config_in_format("toml");
    }
}
