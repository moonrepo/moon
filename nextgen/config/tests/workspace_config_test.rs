mod utils;

use moon_config2::{FilePath, WorkspaceConfig};
use rustc_hash::FxHashMap;
use utils::*;

const FILENAME: &str = ".moon/workspace.yml";

mod workspace_config {
    use super::*;

    #[test]
    #[should_panic(
        expected = "unknown field `unknown`, expected one of `$schema`, `constraints`, `extends`, `generator`, `hasher`, `notifier`, `projects`, `runner`, `telemetry`, `vcs`, `versionConstraint`"
    )]
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
        #[should_panic(expected = "Invalid identifier bad id.")]
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
        #[should_panic(expected = "unknown variant `mercurial`, expected `git` or `svn`")]
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
        #[should_panic(
            expected = "doesn't meet semantic version requirements: unexpected character '@' while parsing major version number"
        )]
        fn errors_on_invalid_req() {
            test_load_config(FILENAME, "versionConstraint: '@1.0.0'", |path| {
                WorkspaceConfig::load_from(path)
            });
        }
    }
}
