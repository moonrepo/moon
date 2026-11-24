use moon_env_var::*;
use rustc_hash::FxHashMap;

mod env_substitutor {
    use super::*;

    #[test]
    fn matches_all_variants() {
        let items = vec![
            // Bash, Zsh, etc
            ("$VAR", "${VAR}", "", "VAR"),
            // Elvish
            ("$E:VAR", "${E:VAR}", "E:", "VAR"),
            // Ion
            ("$env::VAR", "${env::VAR}", "env::", "VAR"),
            // Murex
            ("$ENV.VAR", "${ENV.VAR}", "ENV.", "VAR"),
            // Nu
            ("$env.VAR", "${env.VAR}", "env.", "VAR"),
            // Pwsh
            ("$env:VAR", "${env:VAR}", "env:", "VAR"),
        ];

        let sub = EnvSubstitutor::default();

        for (without_brackets, with_brackets, namespace, name) in items {
            // No brackets
            let without_match = ENV_VAR.captures(without_brackets).unwrap();

            assert_eq!(
                without_match
                    .name("namespace")
                    .map(|cap| cap.as_str())
                    .unwrap_or_default(),
                namespace
            );
            assert_eq!(without_match.name("name").unwrap().as_str(), name);

            assert_eq!(
                sub.get_token_value(
                    without_match
                        .name("namespace")
                        .map(|cap| cap.as_str())
                        .unwrap_or_default(),
                    without_match.name("name").unwrap().as_str()
                ),
                if namespace == "env::" {
                    with_brackets
                } else {
                    without_brackets
                }
            );

            // With brackets
            let with_match = ENV_VAR_BRACKETS.captures(with_brackets).unwrap();

            assert_eq!(
                with_match
                    .name("namespace")
                    .map(|cap| cap.as_str())
                    .unwrap_or_default(),
                namespace
            );
            assert_eq!(with_match.name("name").unwrap().as_str(), name);

            assert_eq!(
                sub.get_token_value(
                    with_match
                        .name("namespace")
                        .map(|cap| cap.as_str())
                        .unwrap_or_default(),
                    with_match.name("name").unwrap().as_str()
                ),
                if namespace == "env::" {
                    with_brackets
                } else {
                    without_brackets
                }
            );

            // With flags
            for flag in ["!", "?"] {
                assert!(
                    ENV_VAR
                        .captures(&format!("${namespace}{name}{flag}"))
                        .is_some()
                );
                assert!(
                    ENV_VAR_BRACKETS
                        .captures(&format!("${{{namespace}{name}{flag}}}"))
                        .is_some()
                );
            }
        }
    }

    #[test]
    fn supports_bracket_fallback() {
        for fallback in ["string", "123", "--arg", "$VAR"] {
            let var = format!("${{ENV_VAR:-{fallback}}}");
            let caps = ENV_VAR_BRACKETS.captures(&var).unwrap();

            assert_eq!(caps.name("fallback").unwrap().as_str(), fallback);
        }
    }

    #[test]
    fn handles_flags_when_missing() {
        let mut sub = EnvSubstitutor::default();

        assert_eq!(sub.substitute("$KEY"), "");
        assert_eq!(sub.substitute("${KEY}"), "");

        assert_eq!(sub.substitute("${KEY!}"), "$KEY");
        assert_eq!(sub.substitute("${KEY?}"), "$KEY");
    }

    #[test]
    fn handles_flags_when_not_missing() {
        let mut envs = FxHashMap::default();
        envs.insert("KEY".to_owned(), Some("value".to_owned()));
        let mut sub = EnvSubstitutor::default().with_local_vars(&envs);

        assert_eq!(sub.substitute("$KEY"), "value");
        assert_eq!(sub.substitute("${KEY}"), "value");

        assert_eq!(sub.substitute("${KEY!}"), "$KEY");
        assert_eq!(sub.substitute("${KEY?}"), "value");
    }

    #[test]
    fn default_flag() {
        let mut sub = EnvSubstitutor::default();

        assert_eq!(sub.substitute("${KEY:default}"), "default");
        assert_eq!(sub.substitute("${KEY:-default}"), "default");
        assert_eq!(sub.substitute("${KEY-default}"), "default");

        let mut envs = FxHashMap::default();
        envs.insert("KEY".to_owned(), Some("value".to_owned()));
        let mut sub = EnvSubstitutor::default().with_local_vars(&envs);

        assert_eq!(sub.substitute("${KEY:default}"), "value");
        assert_eq!(sub.substitute("${KEY:-default}"), "value");
        assert_eq!(sub.substitute("${KEY-default}"), "value");
    }

    #[test]
    fn alternate_flag() {
        let mut sub = EnvSubstitutor::default();

        assert_eq!(sub.substitute("${KEY:+alternate}"), "");
        assert_eq!(sub.substitute("${KEY+alternate}"), "");

        let mut envs = FxHashMap::default();
        envs.insert("KEY".to_owned(), Some("value".to_owned()));
        let mut sub = EnvSubstitutor::default().with_local_vars(&envs);

        assert_eq!(sub.substitute("${KEY:+alternate}"), "alternate");
        assert_eq!(sub.substitute("${KEY+alternate}"), "alternate");
    }

    #[test]
    fn returns_from_local_or_global_bags() {
        let global = GlobalEnvBag::default();
        global.set("SOURCE", "global");
        global.set("GLOBAL", "global");

        let mut local = FxHashMap::default();
        local.insert("SOURCE".to_owned(), Some("local".to_owned()));
        local.insert("LOCAL".to_owned(), Some("local".to_owned()));

        let mut sub = EnvSubstitutor::default()
            .with_local_vars(&local)
            .with_global_vars(&global);

        assert_eq!(sub.substitute("$GLOBAL"), "global");
        assert_eq!(sub.substitute("$LOCAL"), "local");
        assert_eq!(sub.substitute("$SOURCE"), "local");

        // Remove from local
        drop(sub);

        local.remove("SOURCE");

        let mut sub = EnvSubstitutor::default()
            .with_local_vars(&local)
            .with_global_vars(&global);

        assert_eq!(sub.substitute("$SOURCE"), "global");

        // Then remove from global
        global.remove("SOURCE");

        assert_eq!(sub.substitute("$SOURCE"), "");
    }

    #[test]
    fn does_not_consume_env_prefix_chars_as_namespace() {
        let mut envs = FxHashMap::default();
        envs.insert("ENVIRONMENT".to_owned(), Some("production".to_owned()));
        envs.insert("ENVVAR".to_owned(), Some("value".to_owned()));

        let mut sub = EnvSubstitutor::default().with_local_vars(&envs);

        assert_eq!(sub.substitute("$ENVIRONMENT"), "production");
        assert_eq!(sub.substitute("${ENVIRONMENT}"), "production");
        assert_eq!(sub.substitute("$ENVVAR"), "value");
        assert_eq!(sub.substitute("${ENVVAR}"), "value");
    }

    // `$_` is set by Bash to the program it runs
    // https://github.com/moonrepo/moon/issues/2818 (f4a2p6)
    #[test]
    fn requires_a_letter_or_number_in_names() {
        let global = GlobalEnvBag::default();
        global.set("_", "/bin/moon");
        global.set("__", "underscores");
        global.set("_FOO", "foo");
        global.set("__FOO", "foo");
        global.set("_1", "one");

        let mut sub = EnvSubstitutor::default().with_global_vars(&global);

        assert_eq!(sub.substitute("$_"), "$_");
        assert_eq!(sub.substitute("${_}"), "${_}");
        assert_eq!(sub.substitute("$__"), "$__");
        assert_eq!(sub.substitute("${__:-x}"), "${__:-x}");
        assert_eq!(sub.substitute("$__proto__"), "$__proto__");
        assert_eq!(sub.substitute("$_x/file.txt"), "$_x/file.txt");

        assert_eq!(sub.substitute("$_FOO"), "foo");
        assert_eq!(sub.substitute("${__FOO}"), "foo");
        assert_eq!(sub.substitute("$_1"), "one");
    }
}

mod env_scanner {
    use super::*;

    // https://github.com/moonrepo/moon/issues/2818 (f4a2p6)
    #[test]
    fn requires_a_letter_or_number_in_names() {
        let mut scanner = EnvScanner::default();
        scanner.scan("$_ ${__} $__proto__ $_x $_FOO ${__BAR}");

        let mut found = scanner.found.into_iter().collect::<Vec<_>>();
        found.sort();

        assert_eq!(found, vec!["_FOO", "__BAR"]);
    }
}
