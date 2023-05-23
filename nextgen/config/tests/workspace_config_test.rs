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
}
