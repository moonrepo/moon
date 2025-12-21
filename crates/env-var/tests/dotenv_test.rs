use moon_env_var::{DotEnv, GlobalEnvBag, QuoteStyle};
use rustc_hash::FxHashMap;
use std::path::Path;

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
    fn key_prefix() {
        let dot = DotEnv::default();

        assert_eq!(dot.parse_key("ABC").unwrap(), "ABC");
        assert_eq!(dot.parse_key("_ABC").unwrap(), "_ABC");
    }

    #[test]
    #[should_panic(expected = "must start with an alphabetic character or underscore")]
    fn key_prefix_errors_number() {
        DotEnv::default().parse_key("123ABC").unwrap();
    }

    #[test]
    #[should_panic(expected = "must start with an alphabetic character or underscore")]
    fn key_prefix_errors_symbol() {
        DotEnv::default().parse_key("-ABC").unwrap();
    }

    #[test]
    #[should_panic(expected = "must contain alphanumeric characters and underscores")]
    fn key_errors_symbol() {
        DotEnv::default().parse_key("A-BC").unwrap();
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

    #[test]
    fn expand_uses_precedence_local_over_global() {
        let mut env = FxHashMap::default();
        env.insert("SOURCE".to_owned(), Some("local".to_owned()));

        let global = GlobalEnvBag::default();
        global.set("SOURCE", "global");

        assert_eq!(
            DotEnv::default()
                .with_global_vars(&global)
                .substitute_value("KEY", "$SOURCE", &env),
            "local"
        );
    }

    #[test]
    fn expand_bracket_flags_and_fallbacks() {
        let dot = DotEnv::default();

        let mut env = FxHashMap::default();
        env.insert("PRESENT".to_owned(), Some("value".to_owned()));
        env.insert("EMPTY".to_owned(), Some("".to_owned()));

        // ! flag: do not expand
        assert_eq!(dot.substitute_value("KEY", "${PRESENT!}", &env), "$PRESENT");

        // ? flag: expand only if not empty
        assert_eq!(dot.substitute_value("KEY", "${PRESENT?}", &env), "value");
        assert_eq!(dot.substitute_value("KEY", "${EMPTY?}", &env), "$EMPTY");
        assert_eq!(dot.substitute_value("KEY", "${MISSING?}", &env), "$MISSING");

        // : with default -fallback (use when empty/missing)
        assert_eq!(
            dot.substitute_value("KEY", "${EMPTY:-fallback}", &env),
            "fallback"
        );
        assert_eq!(
            dot.substitute_value("KEY", "${PRESENT:-fallback}", &env),
            "value"
        );
        assert_eq!(
            dot.substitute_value("KEY", "${MISSING:-fallback}", &env),
            "fallback"
        );

        // : with alternate +alt (use alt when non-empty)
        assert_eq!(dot.substitute_value("KEY", "${PRESENT:+alt}", &env), "alt");
        assert_eq!(dot.substitute_value("KEY", "${EMPTY:+alt}", &env), "");
        assert_eq!(dot.substitute_value("KEY", "${MISSING:+alt}", &env), "");
    }

    #[test]
    fn expand_non_bracket_after_bracket() {
        let dot = DotEnv::default();
        let env = FxHashMap::default();

        // MISSING resolves to empty for non-bracket; default applies inside brackets
        assert_eq!(
            dot.substitute_value("KEY", "x ${MISSING:-def} y $MISSING", &env),
            "x def y "
        );
    }

    #[test]
    fn expand_with_namespaces() {
        let dot = DotEnv::default();
        let mut env = FxHashMap::default();
        env.insert("NS".to_owned(), Some("ok".to_owned()));

        // Bracketed and non-bracketed namespaces expand using the name portion
        assert_eq!(dot.substitute_value("KEY", "${env:NS}", &env), "ok");
        assert_eq!(dot.substitute_value("KEY", "$env:NS", &env), "ok");

        // Some shells like Ion require brackets; ensure bracket form expands
        assert_eq!(dot.substitute_value("KEY", "${env::NS}", &env), "ok");
    }

    #[test]
    fn load_applies_expansion_and_quote_rules() {
        let dot = DotEnv::default();
        let content =
            "FOO=bar\nBAR=$FOO\nBAZ=${FOO}\nQUX=${FOO:-def}\nQUUX='${FOO}'\nQUUUX=\"$FOO\"";

        let vars = dot.load(content, Path::new("/dev/null")).unwrap();

        assert_eq!(vars.get("FOO").unwrap().as_ref().unwrap(), "bar");
        assert_eq!(vars.get("BAR").unwrap().as_ref().unwrap(), "bar");
        assert_eq!(vars.get("BAZ").unwrap().as_ref().unwrap(), "bar");
        assert_eq!(vars.get("QUX").unwrap().as_ref().unwrap(), "bar");
        assert_eq!(vars.get("QUUX").unwrap().as_ref().unwrap(), "${FOO}");
        assert_eq!(vars.get("QUUUX").unwrap().as_ref().unwrap(), "bar");
    }

    #[test]
    fn load_only_sees_prior_assignments() {
        let dot = DotEnv::default();
        let content = "BAR=$FOO\nFOO=bar";

        let vars = dot.load(content, Path::new("/dev/null")).unwrap();

        assert_eq!(vars.get("FOO").unwrap().as_ref().unwrap(), "bar");
        assert_eq!(vars.get("BAR").unwrap().as_ref().unwrap(), "");
    }

    #[test]
    fn doesnt_expand_in_single_quotes() {
        let vars = DotEnv::default()
            .load("A=a\nB='$A'\nC='${A}'", Path::new("/dev/null"))
            .unwrap();

        assert_eq!(vars.get("A").unwrap().as_ref().unwrap(), "a");
        assert_eq!(vars.get("B").unwrap().as_ref().unwrap(), "$A");
        assert_eq!(vars.get("C").unwrap().as_ref().unwrap(), "${A}");
    }

    #[test]
    fn expands_in_double_quotes() {
        let vars = DotEnv::default()
            .load("A=a\nB=\"$A\"\nC=\"${A}\"", Path::new("/dev/null"))
            .unwrap();

        assert_eq!(vars.get("A").unwrap().as_ref().unwrap(), "a");
        assert_eq!(vars.get("B").unwrap().as_ref().unwrap(), "a");
        assert_eq!(vars.get("C").unwrap().as_ref().unwrap(), "a");
    }

    #[test]
    fn expands_in_unquoted() {
        let vars = DotEnv::default()
            .load("A=a\nB=$A\nC=${A}", Path::new("/dev/null"))
            .unwrap();

        assert_eq!(vars.get("A").unwrap().as_ref().unwrap(), "a");
        assert_eq!(vars.get("B").unwrap().as_ref().unwrap(), "a");
        assert_eq!(vars.get("C").unwrap().as_ref().unwrap(), "a");
    }
}
