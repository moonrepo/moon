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

#[test]
fn contains_subcommand_positive() {
    assert!(contains_subcommand("$(echo hello)"));
    assert!(contains_subcommand("prefix $(command) suffix"));
    assert!(contains_subcommand("$(git rev-parse HEAD)"));
    assert!(contains_subcommand("$(date +%Y%m%d)"));
    assert!(contains_subcommand("multiple $(cmd1) and $(cmd2)"));
}

#[test]
fn contains_subcommand_negative() {
    assert!(!contains_subcommand("$VAR"));
    assert!(!contains_subcommand("${VAR}"));
    assert!(!contains_subcommand("regular text"));
    assert!(!contains_subcommand("$("));
    assert!(!contains_subcommand("$)"));
    assert!(!contains_subcommand("$ (command)"));
}

#[test]
fn subcommand_regex_matches() {
    let matches: Vec<&str> = SUBCOMMAND_SUBSTITUTE
        .find_iter("$(echo hello) and $(date) with text")
        .map(|m| m.as_str())
        .collect();
    
    assert_eq!(matches, vec!["$(echo hello)", "$(date)"]);
}

#[test]
fn contains_arithmetic_expansion_positive() {
    assert!(contains_arithmetic_expansion("$((1 + 2))"));
    assert!(contains_arithmetic_expansion("result=$((10 * 5))"));
    assert!(contains_arithmetic_expansion("$((count++))"));
    assert!(contains_arithmetic_expansion("prefix $((2 ** 8)) suffix"));
}

#[test]
fn contains_arithmetic_expansion_negative() {
    assert!(!contains_arithmetic_expansion("$(command)"));
    assert!(!contains_arithmetic_expansion("$VAR"));
    assert!(!contains_arithmetic_expansion("((expression))"));
    assert!(!contains_arithmetic_expansion("$(("));
    assert!(!contains_arithmetic_expansion("regular text"));
}

#[test]
fn arithmetic_expansion_regex_matches() {
    let matches: Vec<&str> = ARITHMETIC_EXPANSION
        .find_iter("$((1 + 2)) and $((x * y)) calculation")
        .map(|m| m.as_str())
        .collect();
    
    assert_eq!(matches, vec!["$((1 + 2))", "$((x * y))"]);
}

#[test]
fn contains_process_substitution_positive() {
    assert!(contains_process_substitution("diff <(ls dir1) <(ls dir2)"));
    assert!(contains_process_substitution("cat file >(sort)"));
    assert!(contains_process_substitution("cmd <(echo test)"));
    assert!(contains_process_substitution(">(tee output.log)"));
}

#[test]
fn contains_process_substitution_negative() {
    assert!(!contains_process_substitution("$(command)"));
    assert!(!contains_process_substitution("<file.txt"));
    assert!(!contains_process_substitution(">output.txt"));
    assert!(!contains_process_substitution("< (command)"));
    assert!(!contains_process_substitution("regular text"));
}

#[test]
fn process_substitution_regex_matches() {
    let matches: Vec<&str> = PROCESS_SUBSTITUTION
        .find_iter("diff <(sort file1) >(grep pattern)")
        .map(|m| m.as_str())
        .collect();
    
    assert_eq!(matches, vec!["<(sort file1)", ">(grep pattern)"]);
}

#[test]
fn contains_command_substitution_advanced_positive() {
    assert!(contains_command_substitution_advanced("`date`"));
    assert!(contains_command_substitution_advanced("echo `pwd`"));
    assert!(contains_command_substitution_advanced("result=`ls -la`"));
    assert!(contains_command_substitution_advanced("prefix `cmd` suffix"));
}

#[test]
fn contains_command_substitution_advanced_negative() {
    assert!(!contains_command_substitution_advanced("$(command)"));
    assert!(!contains_command_substitution_advanced("'backtick'"));
    assert!(!contains_command_substitution_advanced("`"));
    assert!(!contains_command_substitution_advanced("regular text"));
}

#[test]
fn command_substitution_advanced_regex_matches() {
    let matches: Vec<&str> = COMMAND_SUBSTITUTION_ADVANCED
        .find_iter("echo `date` and `pwd` here")
        .map(|m| m.as_str())
        .collect();
    
    assert_eq!(matches, vec!["`date`", "`pwd`"]);
}

#[test]
fn contains_shell_expansion_comprehensive() {
    // Should detect subcommands
    assert!(contains_shell_expansion("$(echo test)"));
    
    // Should detect arithmetic expansion
    assert!(contains_shell_expansion("$((1 + 2))"));
    
    // Should detect process substitution
    assert!(contains_shell_expansion("<(sort file)"));
    assert!(contains_shell_expansion(">(tee log)"));
    
    // Should detect backticks
    assert!(contains_shell_expansion("`date`"));
    
    // Should not detect regular env vars or text
    assert!(!contains_shell_expansion("$VAR"));
    assert!(!contains_shell_expansion("${VAR}"));
    assert!(!contains_shell_expansion("regular text"));
    
    // Should detect mixed cases
    assert!(contains_shell_expansion("echo $(date) > log-`date +%Y%m%d`.txt"));
    assert!(contains_shell_expansion("result=$((count + 1)) && echo `pwd`"));
}
