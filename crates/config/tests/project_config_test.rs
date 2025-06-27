mod utils;

use moon_common::Id;
use moon_config::{
    ConfigLoader, DependencyConfig, DependencyScope, InputPath, LanguageType, LayerType,
    OwnersPaths, PlatformType, ProjectConfig, ProjectDependsOn, ProjectToolchainEntry, TaskArgs,
    ToolchainPluginConfig,
};
use proto_core::UnresolvedVersionSpec;
use rustc_hash::FxHashMap;
use schematic::schema::IndexMap;
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::Path;
use utils::*;

fn load_config_from_root(root: &Path, source: &str) -> miette::Result<ProjectConfig> {
    ConfigLoader::default().load_project_config_from_source(root, source)
}

mod project_config {
    use super::*;

    #[test]
    #[should_panic(
        expected = "unknown field `unknown`, expected one of `$schema`, `dependsOn`, `docker`, `env`, `fileGroups`, `id`, `language`, `layer`, `type`, `owners`, `platform`, `project`, `stack`, `tags`, `tasks`, `toolchain`, `workspace`"
    )]
    fn error_unknown_field() {
        test_load_config("moon.yml", "unknown: 123", |path| {
            load_config_from_root(path, ".")
        });
    }

    #[test]
    fn loads_defaults() {
        let config = test_load_config("moon.yml", "{}", |path| load_config_from_root(path, "."));

        assert_eq!(config.language, LanguageType::Unknown);
        assert_eq!(config.layer, LayerType::Unknown);
    }

    #[test]
    fn can_use_references() {
        let config = test_load_config(
            "moon.yml",
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
            |path| load_config_from_root(path, "."),
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
            "moon.yml",
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
            |path| load_config_from_root(path, "."),
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
            let config = test_load_config("moon.yml", "dependsOn: ['a', 'b', 'c']", |path| {
                load_config_from_root(path, ".")
            });

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
                "moon.yml",
                r"
dependsOn:
  - id: 'a'
    scope: 'development'
  - id: 'b'
    scope: 'production'",
                |path| load_config_from_root(path, "."),
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
                "moon.yml",
                r"
dependsOn:
  - 'a'
  - id: 'b'
    scope: 'production'",
                |path| load_config_from_root(path, "."),
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
                "moon.yml",
                r"
dependsOn:
  - id: 'a'
    scope: 'invalid'
",
                |path| load_config_from_root(path, "."),
            );
        }
    }

    mod file_groups {
        use super::*;

        #[test]
        fn groups_into_correct_enums() {
            let config = test_load_config(
                "moon.yml",
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
                |path| load_config_from_root(path, "."),
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
            let config = test_load_config("moon.yml", "language: rust", |path| {
                load_config_from_root(path, ".")
            });

            assert_eq!(config.language, LanguageType::Rust);
        }

        #[test]
        fn unsupported_variant_becomes_other() {
            let config = test_load_config("moon.yml", "language: dotnet", |path| {
                load_config_from_root(path, ".")
            });

            assert_eq!(config.language, LanguageType::Other(Id::raw("dotnet")));
        }
    }

    mod owners {
        use super::*;

        #[test]
        fn loads_defaults() {
            let config = test_load_config("moon.yml", "owners: {}", |path| {
                load_config_from_root(path, ".")
            });

            assert_eq!(config.owners.custom_groups, FxHashMap::default());
            assert_eq!(config.owners.default_owner, None);
            assert!(!config.owners.optional);
            assert_eq!(config.owners.paths, OwnersPaths::List(vec![]));
            assert_eq!(config.owners.required_approvals, None);
        }

        #[test]
        fn can_set_values() {
            let config = test_load_config(
                "moon.yml",
                r"
owners:
  customGroups:
    foo: [a, b, c]
    bar: [x, y, z]
  defaultOwner: x
  optional: true
  requiredApprovals: 2
",
                |path| load_config_from_root(path, "."),
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
            assert_eq!(config.owners.required_approvals, Some(2));
        }

        #[test]
        fn can_set_paths_as_list() {
            let config = test_load_config(
                "moon.yml",
                r"
owners:
  defaultOwner: x
  paths:
    - file.txt
    - dir/**/*
",
                |path| load_config_from_root(path, "."),
            );

            assert_eq!(
                config.owners.paths,
                OwnersPaths::List(vec!["file.txt".into(), "dir/**/*".into()])
            );
        }

        #[test]
        fn can_set_paths_as_map() {
            let config = test_load_config(
                "moon.yml",
                r"
owners:
  paths:
    'file.txt': [a, b]
    'dir/**/*': [c, d]
",
                |path| load_config_from_root(path, "."),
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
                "moon.yml",
                r"
owners:
  paths:
    - file.txt
    - dir/**/*
",
                |path| load_config_from_root(path, "."),
            );
        }

        #[test]
        #[should_panic(
            expected = "a default owner is required when defining an empty list of owners"
        )]
        fn errors_on_paths_map_empty_owner() {
            test_load_config(
                "moon.yml",
                r"
owners:
  paths:
    'file.txt': []
",
                |path| load_config_from_root(path, "."),
            );
        }
    }

    mod project {
        use super::*;
        use serde_json::Value;

        #[test]
        #[should_panic(expected = "must not be empty")]
        fn errors_if_empty() {
            test_load_config("moon.yml", "project: {}", |path| {
                load_config_from_root(path, ".")
            });
        }

        #[test]
        fn can_set_only_description() {
            let config = test_load_config(
                "moon.yml",
                r"
project:
  description: 'Text'
",
                |path| load_config_from_root(path, "."),
            );

            let meta = config.project.unwrap();

            assert_eq!(meta.description, "Text");
        }

        #[test]
        fn can_set_all() {
            let config = test_load_config(
                "moon.yml",
                r"
project:
  name: Name
  description: Description
  owner: team
  maintainers: [a, b, c]
  channel: '#abc'
",
                |path| load_config_from_root(path, "."),
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
                "moon.yml",
                r"
project:
  description: 'Test'
  metadata:
    bool: true
    string: 'abc'
",
                |path| load_config_from_root(path, "."),
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
                "moon.yml",
                r"
project:
  description: Description
  channel: abc
",
                |path| load_config_from_root(path, "."),
            );
        }
    }

    mod tags {
        use super::*;

        #[test]
        fn can_set_tags() {
            let config = test_load_config(
                "moon.yml",
                r"
tags:
  - normal
  - camelCase
  - kebab-case
  - snake_case
  - dot.case
  - slash/case
",
                |path| load_config_from_root(path, "."),
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
            test_load_config("moon.yml", "tags: ['foo bar']", |path| {
                load_config_from_root(path, ".")
            });
        }
    }

    mod tasks {
        use super::*;

        #[test]
        fn supports_id_patterns() {
            let config = test_load_config(
                "moon.yml",
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
                |path| load_config_from_root(path, "."),
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
                "moon.yml",
                r"
tasks:
  base:
    command: 'base'
  extender:
    extends: 'base'
    args: '--more'
",
                |path| load_config_from_root(path, "."),
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
                "moon.yml",
                r"
toolchain:
  node:
    version: '18.0.0'
  typescript:
    routeOutDirToCache: true
",
                |path| load_config_from_root(path, "."),
            );

            assert!(config.toolchain.node.is_some());
            assert!(config.toolchain.rust.is_none());

            assert_eq!(
                config.toolchain.node.unwrap().version,
                Some(UnresolvedVersionSpec::parse("18.0.0").unwrap())
            );

            if let ProjectToolchainEntry::Config(ts) =
                config.toolchain.plugins.get("typescript").unwrap()
            {
                assert_eq!(
                    ts.config.get("routeOutDirToCache").unwrap(),
                    &Value::Bool(true),
                );
            }
        }

        #[test]
        fn can_disable_with_null() {
            let config = test_load_config(
                "moon.yml",
                r"
toolchain:
    example: null
",
                |path| load_config_from_root(path, "."),
            );

            assert_eq!(
                config.toolchain.plugins.get("example").unwrap(),
                &ProjectToolchainEntry::Disabled
            );
        }

        #[test]
        fn can_disable_with_false() {
            let config = test_load_config(
                "moon.yml",
                r"
toolchain:
    example: false
",
                |path| load_config_from_root(path, "."),
            );

            assert_eq!(
                config.toolchain.plugins.get("example").unwrap(),
                &ProjectToolchainEntry::Enabled(false)
            );
        }

        #[test]
        fn can_enable_with_true() {
            let config = test_load_config(
                "moon.yml",
                r"
toolchain:
    example: true
",
                |path| load_config_from_root(path, "."),
            );

            assert_eq!(
                config.toolchain.plugins.get("example").unwrap(),
                &ProjectToolchainEntry::Enabled(true)
            );
        }

        #[test]
        fn can_set_customg_config() {
            let config = test_load_config(
                "moon.yml",
                r"
toolchain:
    example:
        version: '1.2.3'
        custom: true
",
                |path| load_config_from_root(path, "."),
            );

            assert_eq!(
                config.toolchain.plugins.get("example").unwrap(),
                &ProjectToolchainEntry::Config(ToolchainPluginConfig {
                    version: Some(UnresolvedVersionSpec::parse("1.2.3").unwrap()),
                    config: BTreeMap::from_iter([("custom".into(), serde_json::Value::Bool(true))]),
                    ..Default::default()
                })
            );
        }
    }

    mod workspace {
        use super::*;

        #[test]
        fn can_set_settings() {
            let config = test_load_config(
                "moon.yml",
                r"
workspace:
  inheritedTasks:
    exclude: [a]
    include: [b]
    rename:
      c: d
",
                |path| load_config_from_root(path, "."),
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

    mod pkl {
        use super::*;
        use moon_common::Id;
        use moon_config::*;
        use starbase_sandbox::locate_fixture;
        use std::collections::BTreeMap;

        #[test]
        #[allow(deprecated)]
        fn loads_pkl() {
            let config = test_config(locate_fixture("pkl"), |path| {
                ConfigLoader::default().load_project_config(path)
            });

            assert_eq!(
                config,
                ProjectConfig {
                    depends_on: vec![
                        ProjectDependsOn::String(Id::raw("a")),
                        ProjectDependsOn::Object(DependencyConfig {
                            id: Id::raw("b"),
                            scope: DependencyScope::Build,
                            source: DependencySource::Implicit,
                            via: None
                        })
                    ],
                    docker: ProjectDockerConfig {
                        file: ProjectDockerFileConfig {
                            build_task: Some(Id::raw("build")),
                            image: Some("node:latest".into()),
                            start_task: Some(Id::raw("start")),
                        },
                        scaffold: ProjectDockerScaffoldConfig {
                            include: vec![GlobPath("*.js".into())]
                        }
                    },
                    env: FxHashMap::from_iter([("KEY".into(), "value".into())]),
                    file_groups: FxHashMap::from_iter([
                        (
                            Id::raw("sources"),
                            vec![InputPath::ProjectGlob("src/**/*".into())]
                        ),
                        (
                            Id::raw("tests"),
                            vec![InputPath::WorkspaceGlob("**/*.test.*".into())]
                        )
                    ]),
                    id: Some(Id::raw("custom-id")),
                    language: LanguageType::Rust,
                    owners: OwnersConfig {
                        custom_groups: FxHashMap::default(),
                        default_owner: Some("owner".into()),
                        optional: true,
                        paths: OwnersPaths::List(vec!["dir/".into(), "file.txt".into()]),
                        required_approvals: Some(5)
                    },
                    platform: Some(PlatformType::Node),
                    project: Some(ProjectMetadataConfig {
                        name: Some("Name".into()),
                        description: "Does something".into(),
                        owner: Some("team".into()),
                        maintainers: vec![],
                        channel: Some("#team".into()),
                        metadata: FxHashMap::from_iter([
                            ("bool".into(), serde_json::Value::Bool(true)),
                            ("string".into(), serde_json::Value::String("abc".into()))
                        ]),
                    }),
                    stack: StackType::Frontend,
                    tags: vec![Id::raw("a"), Id::raw("b"), Id::raw("c")],
                    tasks: BTreeMap::default(),
                    toolchain: ProjectToolchainConfig {
                        deno: Some(ProjectToolchainCommonToolConfig {
                            version: Some(UnresolvedVersionSpec::parse("1.2.3").unwrap()),
                        }),
                        plugins: FxHashMap::from_iter([(
                            Id::raw("typescript"),
                            ProjectToolchainEntry::Config(ToolchainPluginConfig {
                                config: BTreeMap::from_iter([(
                                    "includeSharedTypes".into(),
                                    serde_json::Value::Bool(true)
                                )]),
                                ..Default::default()
                            })
                        )]),
                        ..Default::default()
                    },
                    layer: LayerType::Library,
                    workspace: ProjectWorkspaceConfig {
                        inherited_tasks: ProjectWorkspaceInheritedTasksConfig {
                            exclude: vec![Id::raw("build")],
                            include: Some(vec![Id::raw("test")]),
                            rename: FxHashMap::from_iter([(Id::raw("old"), Id::raw("new"))])
                        }
                    },
                    ..Default::default()
                }
            );
        }
    }
}
