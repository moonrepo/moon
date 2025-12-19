use moon_env_var::DotEnv;
use moon_env_var::QuoteStyle;

mod dotenv {
    use super::*;

    #[test]
    #[should_panic(expected = "Missing `=` in environment variable assignment.")]
    fn line_errors_no_equals() {
        let dot = DotEnv::default();

        dot.parse_line("KEY").unwrap();
    }

    #[test]
    #[should_panic(expected = "Empty environment variable key.")]
    fn line_errors_no_key() {
        let dot = DotEnv::default();

        dot.parse_line("=value").unwrap();
    }

    #[test]
    fn line_empty() {
        let dot = DotEnv::default();

        assert_eq!(dot.parse_line("").unwrap(), None);
        assert_eq!(dot.parse_line("   ").unwrap(), None);
    }

    #[test]
    fn line_comment() {
        let dot = DotEnv::default();

        assert_eq!(dot.parse_line("# comment").unwrap(), None);
        assert_eq!(dot.parse_line(" # comment").unwrap(), None);
        assert_eq!(dot.parse_line("# comment KEY=value").unwrap(), None);
    }

    #[test]
    fn line_export_prefix() {
        let dot = DotEnv::default();

        assert_eq!(
            dot.parse_line("export KEY=value").unwrap(),
            Some(("KEY".to_owned(), "value".to_owned(), QuoteStyle::Unquoted))
        );
    }

    #[test]
    fn line_comment_suffix() {
        let dot = DotEnv::default();

        // unquoted
        assert_eq!(
            dot.parse_line("KEY=value # comment").unwrap(),
            Some(("KEY".to_owned(), "value".to_owned(), QuoteStyle::Unquoted))
        );
        assert_eq!(
            dot.parse_line("KEY=value# comment").unwrap(),
            Some((
                "KEY".to_owned(),
                "value# comment".to_owned(),
                QuoteStyle::Unquoted
            ))
        );

        // single
        assert_eq!(
            dot.parse_line("KEY='value' # comment").unwrap(),
            Some(("KEY".to_owned(), "value".to_owned(), QuoteStyle::Single))
        );
        assert_eq!(
            dot.parse_line("KEY='value # comment'").unwrap(),
            Some((
                "KEY".to_owned(),
                "value # comment".to_owned(),
                QuoteStyle::Single
            ))
        );
        assert_eq!(
            dot.parse_line("KEY='value# comment'").unwrap(),
            Some((
                "KEY".to_owned(),
                "value# comment".to_owned(),
                QuoteStyle::Single
            ))
        );

        // double
        assert_eq!(
            dot.parse_line("KEY=\"value\" # comment").unwrap(),
            Some(("KEY".to_owned(), "value".to_owned(), QuoteStyle::Double))
        );
        assert_eq!(
            dot.parse_line("KEY=\"value # comment\"").unwrap(),
            Some((
                "KEY".to_owned(),
                "value # comment".to_owned(),
                QuoteStyle::Double
            ))
        );
        assert_eq!(
            dot.parse_line("KEY=\"value# comment\"").unwrap(),
            Some((
                "KEY".to_owned(),
                "value# comment".to_owned(),
                QuoteStyle::Double
            ))
        );
    }

    #[test]
    fn value_unquoted() {
        let dot = DotEnv::default();

        assert_eq!(
            dot.parse_value("").unwrap(),
            ("".to_owned(), QuoteStyle::Unquoted)
        );
        assert_eq!(
            dot.parse_value("abc").unwrap(),
            ("abc".to_owned(), QuoteStyle::Unquoted)
        );
        assert_eq!(
            dot.parse_value("a b c").unwrap(),
            ("a b c".to_owned(), QuoteStyle::Unquoted)
        );
        assert_eq!(
            dot.parse_value(" abc ").unwrap(),
            ("abc".to_owned(), QuoteStyle::Unquoted)
        );
    }

    #[test]
    fn value_single_quote() {
        let dot = DotEnv::default();

        assert_eq!(
            dot.parse_value("''").unwrap(),
            ("".to_owned(), QuoteStyle::Single)
        );
        assert_eq!(
            dot.parse_value("'abc'").unwrap(),
            ("abc".to_owned(), QuoteStyle::Single)
        );
        assert_eq!(
            dot.parse_value("'a b c'").unwrap(),
            ("a b c".to_owned(), QuoteStyle::Single)
        );
        assert_eq!(
            dot.parse_value("' abc '").unwrap(),
            (" abc ".to_owned(), QuoteStyle::Single)
        );
        assert_eq!(
            dot.parse_value("'a\nc'").unwrap(),
            ("a\nc".to_owned(), QuoteStyle::Single)
        );
        assert_eq!(
            dot.parse_value("'let\'s go'").unwrap(),
            ("let's go".to_owned(), QuoteStyle::Single)
        );
        assert_eq!(
            dot.parse_value("'let\"s go'").unwrap(),
            ("let\"s go".to_owned(), QuoteStyle::Single)
        );
    }

    #[test]
    fn value_double_quote() {
        let dot = DotEnv::default();

        assert_eq!(
            dot.parse_value("\"\"").unwrap(),
            ("".to_owned(), QuoteStyle::Double)
        );
        assert_eq!(
            dot.parse_value("\"abc\"").unwrap(),
            ("abc".to_owned(), QuoteStyle::Double)
        );
        assert_eq!(
            dot.parse_value("\"a b c\"").unwrap(),
            ("a b c".to_owned(), QuoteStyle::Double)
        );
        assert_eq!(
            dot.parse_value("\" abc \"").unwrap(),
            (" abc ".to_owned(), QuoteStyle::Double)
        );
        assert_eq!(
            dot.parse_value("\"a\nc\"").unwrap(),
            ("a\nc".to_owned(), QuoteStyle::Double)
        );
        assert_eq!(
            dot.parse_value("\"let's go\"").unwrap(),
            ("let's go".to_owned(), QuoteStyle::Double)
        );
        assert_eq!(
            dot.parse_value("\"let\"s go\"").unwrap(),
            ("let\"s go".to_owned(), QuoteStyle::Double)
        );
    }
}
