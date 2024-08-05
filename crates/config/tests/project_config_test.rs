mod utils;

use moon_common::{consts::CONFIG_PROJECT_FILENAME_YML, Id};
use moon_config::{
    DependencyConfig, DependencyScope, InputPath, LanguageType, OwnersPaths, PlatformType,
    ProjectConfig, ProjectDependsOn, ProjectType, TaskArgs,
};
use proto_core::UnresolvedVersionSpec;
use rustc_hash::FxHashMap;
use schematic::schema::IndexMap;
use utils::*;

mod project_config {
    use super::*;

    #[test]
    #[should_panic(
        expected = "unknown field `unknown`, expected one of `$schema`, `dependsOn`, `docker`, `env`, `fileGroups`, `id`, `language`, `owners`, `platform`, `project`, `stack`, `tags`, `tasks`, `toolchain`, `type`, `workspace`"
    )]
    fn error_unknown_field() {
        test_load_config(CONFIG_PROJECT_FILENAME_YML, "unknown: 123", |path| {
            ProjectConfig::load_from(path, ".")
        });
    }

    #[test]
    fn loads_defaults() {
        let config = test_load_config(CONFIG_PROJECT_FILENAME_YML, "{}", |path| {
            ProjectConfig::load_from(path, ".")
        });

        assert_eq!(config.language, LanguageType::Unknown);
        assert_eq!(config.type_of, ProjectType::Unknown);
    }

    #[test]
    fn can_use_references() {
        let config = test_load_config(
            CONFIG_PROJECT_FILENAME_YML,
            r"
tasks:
  build: &webpack
    command: 'webpack'
    inputs:
      - 'src/**/*'
  start:
    <<: *webpack
    args: 'serve'
",
            |path| ProjectConfig::load_from(path, "."),
        );

        let build = config.tasks.get("build").unwrap();

        assert_eq!(build.command, TaskArgs::String("webpack".to_owned()));
        assert_eq!(build.args, TaskArgs::None);
        assert_eq!(
            build.inputs,
            Some(vec![InputPath::ProjectGlob("src/**/*".to_owned())])
        );

        let start = config.tasks.get("start").unwrap();

        assert_eq!(start.command, TaskArgs::String("webpack".to_owned()));
        assert_eq!(start.args, TaskArgs::String("serve".to_owned()));
        assert_eq!(
            start.inputs,
            Some(vec![InputPath::ProjectGlob("src/**/*".to_owned())])
        );
    }

    // TODO: fix this in schematic?
    #[test]
    #[should_panic(expected = "unknown field `_webpack`")]
    fn can_use_references_from_root() {
        let config = test_load_config(
            CONFIG_PROJECT_FILENAME_YML,
            r"
_webpack: &webpack
    command: 'webpack'
    inputs:
      - 'src/**/*'

tasks:
  build: *webpack
  start:
    <<: *webpack
    args: 'serve'
",
            |path| ProjectConfig::load_from(path, "."),
        );

        let build = config.tasks.get("build").unwrap();

        assert_eq!(build.command, TaskArgs::String("webpack".to_owned()));
        assert_eq!(build.args, TaskArgs::None);
        assert_eq!(
            build.inputs,
            Some(vec![InputPath::ProjectGlob("src/**/*".to_owned())])
        );

        let start = config.tasks.get("start").unwrap();

        assert_eq!(start.command, TaskArgs::String("webpack".to_owned()));
        assert_eq!(start.args, TaskArgs::String("serve".to_owned()));
        assert_eq!(
            start.inputs,
            Some(vec![InputPath::ProjectGlob("src/**/*".to_owned())])
        );
    }

    mod depends_on {
        use super::*;

        #[test]
        fn supports_list_of_strings() {
            let config = test_load_config(
                CONFIG_PROJECT_FILENAME_YML,
                "dependsOn: ['a', 'b', 'c']",
                |path| ProjectConfig::load_from(path, "."),
            );

            assert_eq!(
                config.depends_on,
                vec![
                    ProjectDependsOn::String(Id::raw("a")),
                    ProjectDependsOn::String(Id::raw("b")),
                    ProjectDependsOn::String(Id::raw("c"))
                ]
            );
        }

        #[test]
        fn supports_list_of_objects() {
            let config = test_load_config(
                CONFIG_PROJECT_FILENAME_YML,
                r"
dependsOn:
  - id: 'a'
    scope: 'development'
  - id: 'b'
    scope: 'production'",
                |path| ProjectConfig::load_from(path, "."),
            );

            assert_eq!(
                config.depends_on,
                vec![
                    ProjectDependsOn::Object(DependencyConfig {
                        id: Id::raw("a"),
                        scope: DependencyScope::Development,
                        ..DependencyConfig::default()
                    }),
                    ProjectDependsOn::Object(DependencyConfig {
                        id: Id::raw("b"),
                        scope: DependencyScope::Production,
                        ..DependencyConfig::default()
                    })
                ]
            );
        }

        #[test]
        fn supports_list_of_strings_and_objects() {
            let config = test_load_config(
                CONFIG_PROJECT_FILENAME_YML,
                r"
dependsOn:
  - 'a'
  - id: 'b'
    scope: 'production'",
                |path| ProjectConfig::load_from(path, "."),
            );

            assert_eq!(
                config.depends_on,
                vec![
                    ProjectDependsOn::String(Id::raw("a")),
                    ProjectDependsOn::Object(DependencyConfig {
                        id: Id::raw("b"),
                        scope: DependencyScope::Production,
                        ..DependencyConfig::default()
                    })
                ]
            );
        }

        #[test]
        #[should_panic(expected = "expected a project name or dependency config object")]
        fn errors_on_invalid_object_scope() {
            test_load_config(
                CONFIG_PROJECT_FILENAME_YML,
                r"
dependsOn:
  - id: 'a'
    scope: 'invalid'
",
                |path| ProjectConfig::load_from(path, "."),
            );
        }
    }

    mod file_groups {
        use super::*;

        #[test]
        fn groups_into_correct_enums() {
            let config = test_load_config(
                CONFIG_PROJECT_FILENAME_YML,
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
                |path| ProjectConfig::load_from(path, "."),
            );

            assert_eq!(
                config.file_groups,
                FxHashMap::from_iter([
                    (
                        Id::raw("files"),
                        vec![
                            InputPath::WorkspaceFile("ws/relative".into()),
                            InputPath::ProjectFile("proj/relative".into())
                        ]
                    ),
                    (
                        Id::raw("globs"),
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

    mod language {
        use super::*;

        #[test]
        fn supports_variant() {
            let config = test_load_config(CONFIG_PROJECT_FILENAME_YML, "language: rust", |path| {
                ProjectConfig::load_from(path, ".")
            });

            assert_eq!(config.language, LanguageType::Rust);
        }

        #[test]
        fn unsupported_variant_becomes_other() {
            let config = test_load_config(CONFIG_PROJECT_FILENAME_YML, "language: dotnet", |path| {
                ProjectConfig::load_from(path, ".")
            });

            assert_eq!(config.language, LanguageType::Other(Id::raw("dotnet")));
        }
    }

    mod platform {
        use super::*;

        #[test]
        fn supports_variant() {
            let config = test_load_config(CONFIG_PROJECT_FILENAME_YML, "platform: rust", |path| {
                ProjectConfig::load_from(path, ".")
            });

            assert_eq!(config.platform, Some(PlatformType::Rust));
        }

        #[test]
        #[should_panic(
            expected = "unknown variant `perl`, expected one of `bun`, `deno`, `node`, `rust`, `system`, `unknown`"
        )]
        fn errors_on_invalid_variant() {
            test_load_config(CONFIG_PROJECT_FILENAME_YML, "platform: perl", |path| {
                ProjectConfig::load_from(path, ".")
            });
        }
    }

    mod owners {
        use super::*;

        #[test]
        fn loads_defaults() {
            let config = test_load_config(CONFIG_PROJECT_FILENAME_YML, "owners: {}", |path| {
                ProjectConfig::load_from(path, ".")
            });

            assert_eq!(config.owners.custom_groups, FxHashMap::default());
            assert_eq!(config.owners.default_owner, None);
            assert!(!config.owners.optional);
            assert_eq!(config.owners.paths, OwnersPaths::List(vec![]));
            assert_eq!(config.owners.required_approvals, 1);
        }

        #[test]
        fn can_set_values() {
            let config = test_load_config(
                CONFIG_PROJECT_FILENAME_YML,
                r"
owners:
  customGroups:
    foo: [a, b, c]
    bar: [x, y, z]
  defaultOwner: x
  optional: true
  requiredApprovals: 2
",
                |path| ProjectConfig::load_from(path, "."),
            );

            assert_eq!(
                config.owners.custom_groups,
                FxHashMap::from_iter([
                    ("foo".into(), vec!["a".into(), "b".into(), "c".into()]),
                    ("bar".into(), vec!["x".into(), "y".into(), "z".into()]),
                ])
            );
            assert_eq!(config.owners.default_owner, Some("x".to_string()));
            assert!(config.owners.optional);
            assert_eq!(config.owners.required_approvals, 2);
        }

        #[test]
        fn can_set_paths_as_list() {
            let config = test_load_config(
                CONFIG_PROJECT_FILENAME_YML,
                r"
owners:
  defaultOwner: x
  paths:
    - file.txt
    - dir/**/*
",
                |path| ProjectConfig::load_from(path, "."),
            );

            assert_eq!(
                config.owners.paths,
                OwnersPaths::List(vec!["file.txt".into(), "dir/**/*".into()])
            );
        }

        #[test]
        fn can_set_paths_as_map() {
            let config = test_load_config(
                CONFIG_PROJECT_FILENAME_YML,
                r"
owners:
  paths:
    'file.txt': [a, b]
    'dir/**/*': [c, d]
",
                |path| ProjectConfig::load_from(path, "."),
            );

            assert_eq!(
                config.owners.paths,
                OwnersPaths::Map(IndexMap::from_iter([
                    ("file.txt".into(), vec!["a".into(), "b".into()]),
                    ("dir/**/*".into(), vec!["c".into(), "d".into()]),
                ]))
            );
        }

        #[test]
        #[should_panic(expected = "a default owner is required when defining a list of paths")]
        fn errors_on_paths_list_empty_owner() {
            test_load_config(
                CONFIG_PROJECT_FILENAME_YML,
                r"
owners:
  paths:
    - file.txt
    - dir/**/*
",
                |path| ProjectConfig::load_from(path, "."),
            );
        }

        #[test]
        #[should_panic(
            expected = "a default owner is required when defining an empty list of owners"
        )]
        fn errors_on_paths_map_empty_owner() {
            test_load_config(
                CONFIG_PROJECT_FILENAME_YML,
                r"
owners:
  paths:
    'file.txt': []
",
                |path| ProjectConfig::load_from(path, "."),
            );
        }

        #[test]
        #[should_panic(expected = "at least 1 approver is required")]
        fn errors_if_approvers_zero() {
            test_load_config(
                CONFIG_PROJECT_FILENAME_YML,
                r"
owners:
  requiredApprovals: 0
",
                |path| ProjectConfig::load_from(path, "."),
            );
        }
    }

    mod project {
        use super::*;
        use serde_json::Value;

        #[test]
        #[should_panic(expected = "must not be empty")]
        fn errors_if_empty() {
            test_load_config(CONFIG_PROJECT_FILENAME_YML, "project: {}", |path| {
                ProjectConfig::load_from(path, ".")
            });
        }

        #[test]
        fn can_set_only_description() {
            let config = test_load_config(
                CONFIG_PROJECT_FILENAME_YML,
                r"
project:
  description: 'Text'
",
                |path| ProjectConfig::load_from(path, "."),
            );

            let meta = config.project.unwrap();

            assert_eq!(meta.description, "Text");
        }

        #[test]
        fn can_set_all() {
            let config = test_load_config(
                CONFIG_PROJECT_FILENAME_YML,
                r"
project:
  name: Name
  description: Description
  owner: team
  maintainers: [a, b, c]
  channel: '#abc'
",
                |path| ProjectConfig::load_from(path, "."),
            );

            let meta = config.project.unwrap();

            assert_eq!(meta.name.unwrap(), "Name");
            assert_eq!(meta.description, "Description");
            assert_eq!(meta.owner.unwrap(), "team");
            assert_eq!(meta.maintainers, vec!["a", "b", "c"]);
            assert_eq!(meta.channel.unwrap(), "#abc");
        }

        #[test]
        fn can_set_custom_fields() {
            let config = test_load_config(
                CONFIG_PROJECT_FILENAME_YML,
                r"
project:
  description: 'Test'
  metadata:
    bool: true
    string: 'abc'
",
                |path| ProjectConfig::load_from(path, "."),
            );

            let meta = config.project.unwrap();

            assert_eq!(
                meta.metadata,
                FxHashMap::from_iter([
                    ("bool".into(), Value::Bool(true)),
                    ("string".into(), Value::String("abc".into())),
                ])
            );
        }

        #[test]
        #[should_panic(expected = "must start with a `#`")]
        fn errors_if_channel_no_hash() {
            test_load_config(
                CONFIG_PROJECT_FILENAME_YML,
                r"
project:
  description: Description
  channel: abc
",
                |path| ProjectConfig::load_from(path, "."),
            );
        }
    }

    mod tags {
        use super::*;

        #[test]
        fn can_set_tags() {
            let config = test_load_config(
                CONFIG_PROJECT_FILENAME_YML,
                r"
tags:
  - normal
  - camelCase
  - kebab-case
  - snake_case
  - dot.case
  - slash/case
",
                |path| ProjectConfig::load_from(path, "."),
            );

            assert_eq!(
                config.tags,
                vec![
                    Id::raw("normal"),
                    Id::raw("camelCase"),
                    Id::raw("kebab-case"),
                    Id::raw("snake_case"),
                    Id::raw("dot.case"),
                    Id::raw("slash/case")
                ]
            );
        }

        #[test]
        #[should_panic(expected = "Invalid format for foo bar")]
        fn errors_on_invalid_format() {
            test_load_config(CONFIG_PROJECT_FILENAME_YML, "tags: ['foo bar']", |path| {
                ProjectConfig::load_from(path, ".")
            });
        }
    }

    mod tasks {
        use super::*;

        #[test]
        fn supports_id_patterns() {
            let config = test_load_config(
                CONFIG_PROJECT_FILENAME_YML,
                r"
tasks:
  normal:
    command: 'a'
  kebab-case:
    command: 'b'
  camelCase:
    command: 'c'
  snake_case:
    command: 'd'
  dot.case:
    command: 'e'
  slash/case:
    command: 'f'
",
                |path| ProjectConfig::load_from(path, "."),
            );

            assert!(config.tasks.contains_key("normal"));
            assert!(config.tasks.contains_key("kebab-case"));
            assert!(config.tasks.contains_key("camelCase"));
            assert!(config.tasks.contains_key("snake_case"));
            assert!(config.tasks.contains_key("dot.case"));
            assert!(config.tasks.contains_key("slash/case"));
        }

        #[test]
        fn can_extend_siblings() {
            let config = test_load_config(
                CONFIG_PROJECT_FILENAME_YML,
                r"
tasks:
  base:
    command: 'base'
  extender:
    extends: 'base'
    args: '--more'
",
                |path| ProjectConfig::load_from(path, "."),
            );

            assert!(config.tasks.contains_key("base"));
            assert!(config.tasks.contains_key("extender"));
        }
    }

    mod toolchain {
        use super::*;

        #[test]
        fn can_set_settings() {
            let config = test_load_config(
                CONFIG_PROJECT_FILENAME_YML,
                r"
toolchain:
  node:
    version: '18.0.0'
  typescript:
    disabled: false
    routeOutDirToCache: true
",
                |path| ProjectConfig::load_from(path, "."),
            );

            assert!(config.toolchain.node.is_some());
            assert!(config.toolchain.rust.is_none());

            assert_eq!(
                config.toolchain.node.unwrap().version,
                Some(UnresolvedVersionSpec::parse("18.0.0").unwrap())
            );

            let ts = config.toolchain.typescript.unwrap();

            assert!(!ts.disabled);
            assert_eq!(ts.route_out_dir_to_cache, Some(true));
        }
    }

    mod workspace {
        use super::*;

        #[test]
        fn can_set_settings() {
            let config = test_load_config(
                CONFIG_PROJECT_FILENAME_YML,
                r"
workspace:
  inheritedTasks:
    exclude: [a]
    include: [b]
    rename:
      c: d
",
                |path| ProjectConfig::load_from(path, "."),
            );

            assert_eq!(config.workspace.inherited_tasks.exclude, vec![Id::raw("a")]);
            assert_eq!(
                config.workspace.inherited_tasks.include,
                Some(vec![Id::raw("b")])
            );
            assert_eq!(
                config.workspace.inherited_tasks.rename,
                FxHashMap::from_iter([(Id::raw("c"), Id::raw("d"))])
            );
        }
    }
}
