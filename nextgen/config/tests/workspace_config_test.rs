mod utils;

use moon_config::{ExtensionConfig, FilePath, VcsProvider, WorkspaceConfig, WorkspaceProjects};
use rustc_hash::FxHashMap;
use starbase_sandbox::create_sandbox;
use utils::*;

const FILENAME: &str = ".moon/workspace.yml";

mod workspace_config {
    use super::*;

    #[test]
    #[should_panic(expected = "unknown field `unknown`, expected one of `$schema`")]
    fn error_unknown_field() {
        test_load_config(FILENAME, "unknown: 123", |path| {
            WorkspaceConfig::load_from(path)
        });
    }

    #[test]
    fn loads_defaults() {
        let config = test_load_config(FILENAME, "{}", |path| WorkspaceConfig::load_from(path));

        assert!(config.telemetry);
        assert!(config.version_constraint.is_none());
    }

    mod extends {
        use super::*;

        #[test]
        fn recursive_merges() {
            let sandbox = create_sandbox("extends/workspace");
            let config = test_config(sandbox.path().join("base-2.yml"), |path| {
                WorkspaceConfig::load(sandbox.path(), path)
            });

            assert_eq!(config.runner.cache_lifetime, "3 hours");
            assert!(!config.runner.log_running_command);
            assert_eq!(config.vcs.provider, VcsProvider::Bitbucket);
        }

        #[test]
        #[should_panic(expected = "only file paths and URLs can be extended")]
        fn not_a_url_or_file() {
            test_load_config(FILENAME, "extends: 'random value'", |path| {
                WorkspaceConfig::load_from(path)
            });
        }

        #[test]
        #[should_panic(expected = "only secure URLs can be extended")]
        fn not_a_https_url() {
            test_load_config(
                FILENAME,
                "extends: 'http://domain.com/config.yml'",
                |path| WorkspaceConfig::load_from(path),
            );
        }

        #[test]
        #[should_panic(expected = "invalid format, try a supported extension")]
        fn not_a_yaml_file() {
            test_load_config(FILENAME, "extends: './file.txt'", |path| {
                WorkspaceConfig::load_from(path)
            });
        }

        #[test]
        #[should_panic(expected = "invalid format, try a supported extension")]
        fn not_a_yaml_url() {
            test_load_config(
                FILENAME,
                "extends: 'https://domain.com/config.txt'",
                |path| WorkspaceConfig::load_from(path),
            );
        }
    }

    mod projects {
        use super::*;

        #[test]
        fn supports_sources() {
            let config = test_load_config(
                FILENAME,
                r"
projects:
  app: apps/app
  foo-kebab: ./packages/foo
  barCamel: packages/bar
  baz_snake: ./packages/baz
  qux.dot: packages/qux
  wat/slash: ./packages/wat
",
                |path| WorkspaceConfig::load_from(path),
            );

            match config.projects {
                WorkspaceProjects::Sources(map) => {
                    assert_eq!(
                        map,
                        FxHashMap::from_iter([
                            ("app".into(), "apps/app".into()),
                            ("foo-kebab".into(), "./packages/foo".into()),
                            ("barCamel".into(), "packages/bar".into()),
                            ("baz_snake".into(), "./packages/baz".into()),
                            ("qux.dot".into(), "packages/qux".into()),
                            ("wat/slash".into(), "./packages/wat".into())
                        ]),
                    );
                }
                _ => panic!(),
            };
        }

        #[test]
        #[should_panic(expected = "absolute paths are not supported")]
        fn errors_on_absolute_sources() {
            test_load_config(
                FILENAME,
                r"
projects:
  app: /apps/app
",
                |path| WorkspaceConfig::load_from(path),
            );
        }

        #[test]
        #[should_panic(expected = "parent relative paths are not supported")]
        fn errors_on_parent_sources() {
            test_load_config(
                FILENAME,
                r"
projects:
  app: ../apps/app
",
                |path| WorkspaceConfig::load_from(path),
            );
        }

        #[test]
        #[should_panic(expected = "globs are not supported, expected a literal file path")]
        fn errors_on_glob_in_sources() {
            test_load_config(
                FILENAME,
                r"
projects:
  app: apps/app/*
",
                |path| WorkspaceConfig::load_from(path),
            );
        }

        #[test]
        fn supports_globs() {
            let config = test_load_config(
                FILENAME,
                r"
projects:
  - apps/*
  - packages/*
  - internal
",
                |path| WorkspaceConfig::load_from(path),
            );

            match config.projects {
                WorkspaceProjects::Globs(list) => {
                    assert_eq!(
                        list,
                        vec![
                            "apps/*".to_owned(),
                            "packages/*".to_owned(),
                            "internal".to_owned(),
                        ],
                    );
                }
                _ => panic!(),
            };
        }

        #[test]
        #[should_panic(expected = "absolute paths are not supported")]
        fn errors_on_absolute_globs() {
            test_load_config(
                FILENAME,
                r"
projects:
  - /apps/*
",
                |path| WorkspaceConfig::load_from(path),
            );
        }

        #[test]
        #[should_panic(expected = "parent relative paths are not supported")]
        fn errors_on_parent_globs() {
            test_load_config(
                FILENAME,
                r"
projects:
  - ../apps/*
",
                |path| WorkspaceConfig::load_from(path),
            );
        }

        #[test]
        fn supports_globs_and_projects() {
            let config = test_load_config(
                FILENAME,
                r"
projects:
  sources:
    app: app
  globs:
    - packages/*
",
                |path| WorkspaceConfig::load_from(path),
            );

            match config.projects {
                WorkspaceProjects::Both(cfg) => {
                    assert_eq!(cfg.globs, vec!["packages/*".to_owned()]);
                    assert_eq!(
                        cfg.sources,
                        FxHashMap::from_iter([("app".into(), "app".into())])
                    );
                }
                _ => panic!(),
            };
        }
    }

    mod constraints {
        use super::*;

        #[test]
        fn loads_defaults() {
            let config = test_load_config(FILENAME, "constraints: {}", |path| {
                WorkspaceConfig::load_from(path)
            });

            assert!(config.constraints.enforce_project_type_relationships);
            assert!(config.constraints.tag_relationships.is_empty());
        }

        #[test]
        fn can_set_tags() {
            let config = test_load_config(
                FILENAME,
                r"
constraints:
  tagRelationships:
    id: ['other']
",
                |path| WorkspaceConfig::load_from(path),
            );

            assert!(config.constraints.enforce_project_type_relationships);
            assert_eq!(
                config.constraints.tag_relationships,
                FxHashMap::from_iter([("id".into(), vec!["other".into()])])
            );
        }

        #[test]
        #[should_panic(
            expected = "invalid type: integer `123`, expected struct PartialConstraintsConfig"
        )]
        fn errors_on_invalid_type() {
            test_load_config(FILENAME, "constraints: 123", |path| {
                WorkspaceConfig::load_from(path)
            });
        }

        #[test]
        #[should_panic(expected = "invalid type: string \"abc\", expected a boolean")]
        fn errors_on_invalid_setting_type() {
            test_load_config(
                FILENAME,
                r"
constraints:
  enforceProjectTypeRelationships: abc
",
                |path| WorkspaceConfig::load_from(path),
            );
        }

        #[test]
        #[should_panic(expected = "Invalid format for bad id")]
        fn errors_on_invalid_tag_format() {
            test_load_config(
                FILENAME,
                r"
constraints:
  tagRelationships:
    id: ['bad id']
",
                |path| WorkspaceConfig::load_from(path),
            );
        }
    }

    mod generator {
        use super::*;

        #[test]
        fn loads_defaults() {
            let config = test_load_config(FILENAME, "generator: {}", |path| {
                WorkspaceConfig::load_from(path)
            });

            assert_eq!(
                config.generator.templates,
                vec![FilePath("./templates".into())]
            );
        }

        #[test]
        fn can_set_templates() {
            let config = test_load_config(
                FILENAME,
                r"
generator:
  templates:
    - custom/path
    - ./rel/path
    - ../parent/path
    - /abs/path
",
                |path| WorkspaceConfig::load_from(path),
            );

            assert_eq!(
                config.generator.templates,
                vec![
                    FilePath("custom/path".into()),
                    FilePath("./rel/path".into()),
                    FilePath("../parent/path".into()),
                    FilePath("/abs/path".into())
                ]
            );
        }

        #[test]
        #[should_panic(expected = "globs are not supported, expected a literal file path")]
        fn errors_on_template_glob() {
            test_load_config(
                FILENAME,
                r"
generator:
  templates: ['glob/**/*']
",
                |path| WorkspaceConfig::load_from(path),
            );
        }
    }

    mod hasher {
        use super::*;

        #[test]
        fn loads_defaults() {
            let config = test_load_config(FILENAME, "hasher: {}", |path| {
                WorkspaceConfig::load_from(path)
            });

            assert_eq!(config.hasher.batch_size, 2500);
            assert!(config.hasher.warn_on_missing_inputs);
        }

        #[test]
        fn can_set_settings() {
            let config = test_load_config(
                FILENAME,
                r"
hasher:
  batchSize: 1000
  warnOnMissingInputs: false
",
                |path| WorkspaceConfig::load_from(path),
            );

            assert_eq!(config.hasher.batch_size, 1000);
            assert!(!config.hasher.warn_on_missing_inputs);
        }

        #[test]
        #[should_panic(expected = "unknown variant `unknown`, expected `glob` or `vcs`")]
        fn errors_on_invalid_variant() {
            test_load_config(
                FILENAME,
                r"
hasher:
  walkStrategy: unknown
",
                |path| WorkspaceConfig::load_from(path),
            );
        }
    }

    mod notifier {
        use super::*;

        #[test]
        fn loads_defaults() {
            let config = test_load_config(FILENAME, "notifier: {}", |path| {
                WorkspaceConfig::load_from(path)
            });

            assert!(config.notifier.webhook_url.is_none());
        }

        #[test]
        fn can_set_settings() {
            let config = test_load_config(
                FILENAME,
                r"
notifier:
  webhookUrl: 'https://domain.com/some/url'
",
                |path| WorkspaceConfig::load_from(path),
            );

            assert_eq!(
                config.notifier.webhook_url,
                Some("https://domain.com/some/url".into())
            );
        }

        #[test]
        #[should_panic(expected = "not a valid url: relative URL without a base")]
        fn errors_on_invalid_url() {
            test_load_config(
                FILENAME,
                r"
notifier:
  webhookUrl: 'invalid value'
",
                |path| WorkspaceConfig::load_from(path),
            );
        }

        #[test]
        #[should_panic(expected = "only secure URLs are allowed")]
        fn errors_on_non_https_url() {
            test_load_config(
                FILENAME,
                r"
notifier:
  webhookUrl: 'http://domain.com/some/url'
",
                |path| WorkspaceConfig::load_from(path),
            );
        }
    }

    mod runner {
        use super::*;
        use moon_target::Target;

        #[test]
        fn loads_defaults() {
            let config = test_load_config(FILENAME, "runner: {}", |path| {
                WorkspaceConfig::load_from(path)
            });

            assert_eq!(config.runner.cache_lifetime, "7 days");
            assert!(config.runner.inherit_colors_for_piped_tasks);
        }

        #[test]
        fn can_set_settings() {
            let config = test_load_config(
                FILENAME,
                r"
runner:
  cacheLifetime: 10 hours
  inheritColorsForPipedTasks: false
",
                |path| WorkspaceConfig::load_from(path),
            );

            assert_eq!(config.runner.cache_lifetime, "10 hours");
            assert!(!config.runner.inherit_colors_for_piped_tasks);
        }

        #[test]
        fn can_use_targets() {
            let config = test_load_config(
                FILENAME,
                r"
runner:
  archivableTargets: ['scope:task']
",
                |path| WorkspaceConfig::load_from(path),
            );

            assert_eq!(
                config.runner.archivable_targets,
                vec![Target::new("scope", "task").unwrap()]
            );
        }

        #[test]
        #[should_panic(expected = "Invalid target ~:bad target")]
        fn errors_on_invalid_target() {
            test_load_config(
                FILENAME,
                r"
runner:
  archivableTargets: ['bad target']
",
                |path| WorkspaceConfig::load_from(path),
            );
        }
    }

    mod vcs {
        use super::*;

        #[test]
        fn loads_defaults() {
            let config =
                test_load_config(FILENAME, "vcs: {}", |path| WorkspaceConfig::load_from(path));

            assert_eq!(config.vcs.default_branch, "master");
            assert_eq!(
                config.vcs.remote_candidates,
                vec!["origin".to_string(), "upstream".to_string()]
            );
        }

        #[test]
        fn can_set_settings() {
            let config = test_load_config(
                FILENAME,
                r"
vcs:
  defaultBranch: main
  remoteCandidates: [next]
",
                |path| WorkspaceConfig::load_from(path),
            );

            assert_eq!(config.vcs.default_branch, "main");
            assert_eq!(config.vcs.remote_candidates, vec!["next".to_string()]);
        }

        #[test]
        #[should_panic(expected = "unknown variant `mercurial`, expected `git`")]
        fn errors_on_invalid_manager() {
            test_load_config(
                FILENAME,
                r"
vcs:
  manager: mercurial
",
                |path| WorkspaceConfig::load_from(path),
            );
        }
    }

    mod version_constraint {
        use super::*;

        #[test]
        #[should_panic(expected = "unexpected character '@' while parsing major version number")]
        fn errors_on_invalid_req() {
            test_load_config(FILENAME, "versionConstraint: '@1.0.0'", |path| {
                WorkspaceConfig::load_from(path)
            });
        }
    }

    mod extensions {
        use super::*;
        use proto_core::{Id, PluginLocator};

        #[test]
        #[should_panic(
            expected = "Invalid plugin identifier bad.id, must be a valid kebab-case string"
        )]
        fn errors_invalid_id() {
            test_load_config(
                FILENAME,
                r"
extensions:
    bad.id: 'source:https://domain.com'
",
                |path| WorkspaceConfig::load_from(path),
            );
        }

        #[test]
        #[should_panic(expected = "extensions.id.plugin: Missing plugin scope or location.")]
        fn errors_invalid_locator() {
            test_load_config(
                FILENAME,
                r"
extensions:
    id:
        plugin: 'missing-scope'
",
                |path| WorkspaceConfig::load_from(path),
            );
        }

        #[test]
        #[should_panic(expected = "extensions.id.plugin: this setting is required")]
        fn errors_missing_locator() {
            test_load_config(
                FILENAME,
                r"
extensions:
    id:
        foo: 'bar'
",
                |path| WorkspaceConfig::load_from(path),
            );
        }

        #[test]
        fn can_set_with_object() {
            let config = test_load_config(
                FILENAME,
                r"
extensions:
    test-id:
        plugin: 'source:https://domain.com'
",
                |path| WorkspaceConfig::load_from(path),
            );

            assert_eq!(
                config.extensions,
                FxHashMap::from_iter([(
                    Id::raw("test-id"),
                    ExtensionConfig {
                        config: FxHashMap::default(),
                        plugin: Some(PluginLocator::SourceUrl {
                            url: "https://domain.com".into()
                        }),
                    }
                )])
            );
        }

        #[test]
        fn can_set_additional_object_config() {
            let config = test_load_config(
                FILENAME,
                r"
extensions:
    test-id:
        plugin: 'source:https://domain.com'
        fooBar: 'abc'
        bar-baz: true
",
                |path| WorkspaceConfig::load_from(path),
            );

            assert_eq!(
                config.extensions,
                FxHashMap::from_iter([(
                    Id::raw("test-id"),
                    ExtensionConfig {
                        config: FxHashMap::from_iter([
                            ("fooBar".into(), serde_json::Value::String("abc".into())),
                            ("bar-baz".into(), serde_json::Value::Bool(true)),
                        ]),
                        plugin: Some(PluginLocator::SourceUrl {
                            url: "https://domain.com".into()
                        }),
                    }
                )])
            );
        }
    }
}
