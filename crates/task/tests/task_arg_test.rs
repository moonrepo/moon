use moon_task::TaskArg;

mod task_arg {
    use super::*;

    #[test]
    fn keeps_unquoted_values() {
        let arg = TaskArg::new("--foo=bar");

        assert_eq!(arg.value, "--foo=bar");
        assert_eq!(arg.quoted_value, None);
        assert_eq!(arg.get_value(), "--foo=bar");
    }

    #[test]
    fn unquotes_each_syntax() {
        for (base, value) in [
            ("\"foo bar\"", "foo bar"),
            ("'foo bar'", "foo bar"),
            ("$\"foo bar\"", "foo bar"),
            ("$'foo bar'", "foo bar"),
            ("%(foo bar)", "foo bar"),
            ("r#'foo bar'#", "foo bar"),
        ] {
            let arg = TaskArg::new(base);

            assert_eq!(arg.value, value, "{base}");
            assert_eq!(arg.quoted_value.as_deref(), Some(base), "{base}");
            assert_eq!(arg.get_value(), base, "{base}");
        }
    }

    #[test]
    fn unquotes_empty_values() {
        for base in ["\"\"", "''", "$\"\"", "$''", "%()", "r#''#"] {
            let arg = TaskArg::new(base);

            assert_eq!(arg.value, "", "{base}");
            assert!(arg.is_empty(), "{base}");
            assert!(arg.is_quoted(), "{base}");
        }
    }

    #[test]
    fn only_removes_one_quote_from_each_side() {
        // Escaped quotes at the edges of the value
        assert_eq!(TaskArg::new("\"\\\"foo bar\\\"\"").value, "\\\"foo bar\\\"");

        // Nested delimiters at the edges of the value
        assert_eq!(TaskArg::new("%(foo(bar))").value, "foo(bar)");
        assert_eq!(TaskArg::new("%(%(foo))").value, "%(foo)");
    }
}
