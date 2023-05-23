mod utils;

use moon_common::{consts::CONFIG_PROJECT_FILENAME, Id};
use moon_config2::{
    DependencyScope, FilePath, GlobPath, LanguageType, PlatformType, PortablePath, ProjectConfig,
    ProjectDependsOn, ProjectType,
};
use rustc_hash::FxHashMap;
use utils::*;

mod project_config {
    use super::*;

    #[test]
    #[should_panic(
        expected = "unknown field `unknown`, expected one of `$schema`, `dependsOn`, `env`, `fileGroups`, `language`, `platform`, `project`, `tags`, `tasks`, `toolchain`, `type`, `workspace`"
    )]
    fn error_unknown_field() {
        test_load_config(CONFIG_PROJECT_FILENAME, "unknown: 123", |path| {
            ProjectConfig::load_from(path, ".")
        });
    }

    #[test]
    fn loads_defaults() {
        let config = test_load_config(CONFIG_PROJECT_FILENAME, "{}", |path| {
            ProjectConfig::load_from(path, ".")
        });

        assert_eq!(config.language, LanguageType::Unknown);
        assert_eq!(config.type_of, ProjectType::Unknown);
    }

    mod depends_on {
        use super::*;

        #[test]
        fn supports_list_of_strings() {
            let config = test_load_config(
                CONFIG_PROJECT_FILENAME,
                "dependsOn: ['a', 'b', 'c']",
                |path| ProjectConfig::load_from(path, "."),
            );

            assert_eq!(
                config.depends_on,
                vec![
                    ProjectDependsOn::String("a".into()),
                    ProjectDependsOn::String("b".into()),
                    ProjectDependsOn::String("c".into())
                ]
            );
        }

        #[test]
        fn supports_list_of_objects() {
            let config = test_load_config(
                CONFIG_PROJECT_FILENAME,
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
                    ProjectDependsOn::Object {
                        id: "a".into(),
                        scope: DependencyScope::Development,
                    },
                    ProjectDependsOn::Object {
                        id: "b".into(),
                        scope: DependencyScope::Production,
                    }
                ]
            );
        }

        #[test]
        fn supports_list_of_strings_and_objects() {
            let config = test_load_config(
                CONFIG_PROJECT_FILENAME,
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
                    ProjectDependsOn::String("a".into()),
                    ProjectDependsOn::Object {
                        id: "b".into(),
                        scope: DependencyScope::Production,
                    }
                ]
            );
        }

        #[test]
        #[should_panic(expected = "expected a project name or dependency config object")]
        fn errors_on_invalid_object_scope() {
            test_load_config(
                CONFIG_PROJECT_FILENAME,
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
                CONFIG_PROJECT_FILENAME,
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

    mod language {
        use super::*;

        #[test]
        fn supports_variant() {
            let config = test_load_config(CONFIG_PROJECT_FILENAME, "language: rust", |path| {
                ProjectConfig::load_from(path, ".")
            });

            assert_eq!(config.language, LanguageType::Rust);
        }

        #[test]
        fn unsupported_variant_becomes_other() {
            let config = test_load_config(CONFIG_PROJECT_FILENAME, "language: dotnet", |path| {
                ProjectConfig::load_from(path, ".")
            });

            assert_eq!(config.language, LanguageType::Other(Id::raw("dotnet")));
        }
    }

    mod platform {
        use super::*;

        #[test]
        fn supports_variant() {
            let config = test_load_config(CONFIG_PROJECT_FILENAME, "platform: rust", |path| {
                ProjectConfig::load_from(path, ".")
            });

            assert_eq!(config.platform, Some(PlatformType::Rust));
        }

        #[test]
        #[should_panic(
            expected = "unknown variant `perl`, expected one of `deno`, `node`, `rust`, `system`, `unknown`"
        )]
        fn errors_on_invalid_variant() {
            test_load_config(CONFIG_PROJECT_FILENAME, "platform: perl", |path| {
                ProjectConfig::load_from(path, ".")
            });
        }
    }

    mod project {
        use super::*;

        #[test]
        #[should_panic(expected = "must not be empty")]
        fn errors_if_empty() {
            test_load_config(CONFIG_PROJECT_FILENAME, "project: {}", |path| {
                ProjectConfig::load_from(path, ".")
            });
        }

        #[test]
        fn can_set_only_description() {
            let config = test_load_config(
                CONFIG_PROJECT_FILENAME,
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
                CONFIG_PROJECT_FILENAME,
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
        #[should_panic(expected = "must start with a `#`")]
        fn errors_if_channel_no_hash() {
            test_load_config(
                CONFIG_PROJECT_FILENAME,
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
                CONFIG_PROJECT_FILENAME,
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
            test_load_config(CONFIG_PROJECT_FILENAME, "tags: ['foo bar']", |path| {
                ProjectConfig::load_from(path, ".")
            });
        }
    }

    mod tasks {
        use super::*;
    }

    mod toolchain {
        use super::*;

        #[test]
        fn can_set_settings() {
            let config = test_load_config(
                CONFIG_PROJECT_FILENAME,
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
                Some("18.0.0".to_string())
            );

            let ts = config.toolchain.typescript.unwrap();

            assert_eq!(ts.disabled, false);
            assert_eq!(ts.route_out_dir_to_cache, Some(true));
        }
    }

    mod workspace {
        use super::*;

        #[test]
        fn can_set_settings() {
            let config = test_load_config(
                CONFIG_PROJECT_FILENAME,
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
            assert_eq!(config.workspace.inherited_tasks.include, vec![Id::raw("b")]);
            assert_eq!(
                config.workspace.inherited_tasks.rename,
                FxHashMap::from_iter([(Id::raw("c"), Id::raw("d"))])
            );
        }
    }
}
