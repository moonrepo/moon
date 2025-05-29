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

        for (without_brackets, with_brackets, namespace, name) in items {
            // No brackets
            let without_match = ENV_VAR_SUBSTITUTE.captures(without_brackets).unwrap();

            assert_eq!(
                without_match
                    .name("namespace")
                    .map(|cap| cap.as_str())
                    .unwrap_or_default(),
                namespace
            );
            assert_eq!(without_match.name("name").unwrap().as_str(), name);

            assert_eq!(
                rebuild_env_var(&without_match),
                if namespace == "env::" {
                    with_brackets
                } else {
                    without_brackets
                }
            );

            // With brackets
            let with_match = ENV_VAR_SUBSTITUTE_BRACKETS.captures(with_brackets).unwrap();

            assert_eq!(
                with_match
                    .name("namespace")
                    .map(|cap| cap.as_str())
                    .unwrap_or_default(),
                namespace
            );
            assert_eq!(with_match.name("name").unwrap().as_str(), name);

            assert_eq!(
                rebuild_env_var(&with_match),
                if namespace == "env::" {
                    with_brackets
                } else {
                    without_brackets
                }
            );

            // With flags
            for flag in ["!", "?"] {
                assert!(
                    ENV_VAR_SUBSTITUTE
                        .captures(&format!("${namespace}{name}{flag}"))
                        .is_some()
                );
                assert!(
                    ENV_VAR_SUBSTITUTE_BRACKETS
                        .captures(&format!("${{{namespace}{name}{flag}}}"))
                        .is_some()
                );
            }
        }
    }

    #[test]
    fn supports_bracket_fallback() {
        for fallback in ["string", "123", "--arg", "$VAR"] {
            let var = format!("${{ENV_VAR:{fallback}}}");
            let caps = ENV_VAR_SUBSTITUTE_BRACKETS.captures(&var).unwrap();

            assert_eq!(caps.name("fallback").unwrap().as_str(), fallback);
        }
    }

    #[test]
    fn handles_flags_when_missing() {
        let mut sub = EnvSubstitutor::default();

        assert_eq!(sub.substitute("$KEY"), "$KEY");
        assert_eq!(sub.substitute("${KEY}"), "${KEY}");

        assert_eq!(sub.substitute("$KEY!"), "$KEY");
        assert_eq!(sub.substitute("${KEY!}"), "$KEY");

        assert_eq!(sub.substitute("$KEY?"), "");
        assert_eq!(sub.substitute("${KEY?}"), "");
    }

    #[test]
    fn handles_flags_when_not_missing() {
        let mut envs = FxHashMap::default();
        envs.insert("KEY".to_owned(), "value".to_owned());
        let mut sub = EnvSubstitutor::default().with_local_vars(&envs);

        assert_eq!(sub.substitute("$KEY"), "value");
        assert_eq!(sub.substitute("${KEY}"), "value");

        assert_eq!(sub.substitute("$KEY!"), "$KEY");
        assert_eq!(sub.substitute("${KEY!}"), "$KEY");

        assert_eq!(sub.substitute("$KEY?"), "value");
        assert_eq!(sub.substitute("${KEY?}"), "value");
    }

    #[test]
    fn returns_from_local_or_global_bags() {
        let global = GlobalEnvBag::default();
        global.set("SOURCE", "global");
        global.set("GLOBAL", "global");

        let mut local = FxHashMap::default();
        local.insert("SOURCE".to_owned(), "local".to_owned());
        local.insert("LOCAL".to_owned(), "local".to_owned());

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

        assert_eq!(sub.substitute("$SOURCE"), "$SOURCE");
    }
}
