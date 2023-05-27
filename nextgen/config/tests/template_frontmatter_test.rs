mod utils;

use moon_config::TemplateFrontmatterConfig;
use utils::*;

mod template_frontmatter {
    use super::*;

    #[test]
    #[should_panic(
        expected = "unknown field `title`, expected one of `$schema`, `force`, `to`, `skip`"
    )]
    fn error_unknown_field() {
        test_parse_config("title: test", |code| TemplateFrontmatterConfig::parse(code));
    }

    #[test]
    fn loads_defaults() {
        let config = test_parse_config("", |code| TemplateFrontmatterConfig::parse(code));

        assert!(!config.force);
        assert!(!config.skip);
        assert_eq!(config.to, None);
    }

    #[test]
    fn can_set_force_skip() {
        let config = test_parse_config("force: true\nskip: true", |code| {
            TemplateFrontmatterConfig::parse(code)
        });

        assert!(config.force);
        assert!(config.skip);
    }

    #[test]
    #[should_panic(expected = "invalid type: integer `123`, expected a boolean")]
    fn invalid_force() {
        test_parse_config("force: 123", |code| TemplateFrontmatterConfig::parse(code));
    }

    #[test]
    #[should_panic(expected = "invalid type: string \"abc\", expected a boolean")]
    fn invalid_skip() {
        test_parse_config("skip: abc", |code| TemplateFrontmatterConfig::parse(code));
    }

    #[test]
    fn can_set_to() {
        let config = test_parse_config("to: some/path", |code| {
            TemplateFrontmatterConfig::parse(code)
        });

        assert_eq!(config.to, Some("some/path".into()));
    }

    #[test]
    #[should_panic(expected = "invalid type: boolean `true`, expected a string")]
    fn invalid_to() {
        test_parse_config("to: true", |code| TemplateFrontmatterConfig::parse(code));
    }
}
