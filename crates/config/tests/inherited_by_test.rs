use moon_common::Id;
use moon_config::{
    FilePath, InheritFor, LanguageType, LayerType, OneOrMany, PartialInheritedByConfig,
    PartialInheritedClauseConfig, PartialInheritedConditionConfig, PortablePath, StackType,
};
use starbase_sandbox::create_empty_sandbox;

mod inherited_by {
    use super::*;

    mod clauses {
        use super::*;

        #[test]
        fn doesnt_match_when_empty() {
            let clause = PartialInheritedClauseConfig::default();

            assert!(!clause.matches(&[Id::raw("a")]));
        }

        #[test]
        fn doesnt_match_when_no_values() {
            let clause = PartialInheritedClauseConfig {
                or: Some(OneOrMany::One(Id::raw("a"))),
                ..Default::default()
            };

            assert!(!clause.matches(&[]));
        }

        #[test]
        fn and_matches_one() {
            let clause = PartialInheritedClauseConfig {
                and: Some(OneOrMany::One(Id::raw("a"))),
                ..Default::default()
            };

            assert!(clause.matches(&[Id::raw("a")]));
            assert!(clause.matches(&[Id::raw("a"), Id::raw("b")]));
        }

        #[test]
        fn and_doesnt_match_one() {
            let clause = PartialInheritedClauseConfig {
                and: Some(OneOrMany::One(Id::raw("a"))),
                ..Default::default()
            };

            assert!(!clause.matches(&[Id::raw("b")]));
            assert!(!clause.matches(&[Id::raw("c"), Id::raw("d")]));
        }

        #[test]
        fn and_matches_many() {
            let clause = PartialInheritedClauseConfig {
                and: Some(OneOrMany::Many(vec![Id::raw("a"), Id::raw("b")])),
                ..Default::default()
            };

            assert!(clause.matches(&[Id::raw("a"), Id::raw("b")]));
            assert!(clause.matches(&[Id::raw("a"), Id::raw("b"), Id::raw("c")]));
        }

        #[test]
        fn and_doesnt_match_many() {
            let clause = PartialInheritedClauseConfig {
                and: Some(OneOrMany::Many(vec![Id::raw("a"), Id::raw("b")])),
                ..Default::default()
            };

            assert!(!clause.matches(&[Id::raw("a")]));
            assert!(!clause.matches(&[Id::raw("b")]));
            assert!(!clause.matches(&[Id::raw("c"), Id::raw("b")]));
            assert!(!clause.matches(&[Id::raw("a"), Id::raw("d")]));
            assert!(!clause.matches(&[Id::raw("c"), Id::raw("d")]));
        }

        #[test]
        fn and_works_with_not() {
            let clause = PartialInheritedClauseConfig {
                and: Some(OneOrMany::Many(vec![Id::raw("a"), Id::raw("b")])),
                not: Some(OneOrMany::Many(vec![Id::raw("c")])),
                ..Default::default()
            };

            assert!(clause.matches(&[Id::raw("a"), Id::raw("b")]));

            assert!(!clause.matches(&[Id::raw("a")]));
            assert!(!clause.matches(&[Id::raw("b")]));
            assert!(!clause.matches(&[Id::raw("a"), Id::raw("b"), Id::raw("c")]));
        }

        #[test]
        fn or_matches_one() {
            let clause = PartialInheritedClauseConfig {
                or: Some(OneOrMany::One(Id::raw("a"))),
                ..Default::default()
            };

            assert!(clause.matches(&[Id::raw("a")]));
        }

        #[test]
        fn or_doesnt_match_one() {
            let clause = PartialInheritedClauseConfig {
                or: Some(OneOrMany::One(Id::raw("a"))),
                ..Default::default()
            };

            assert!(!clause.matches(&[Id::raw("b")]));
            assert!(!clause.matches(&[Id::raw("c"), Id::raw("d")]));
        }

        #[test]
        fn or_matches_many() {
            let clause = PartialInheritedClauseConfig {
                or: Some(OneOrMany::Many(vec![Id::raw("a"), Id::raw("b")])),
                ..Default::default()
            };

            assert!(clause.matches(&[Id::raw("a")]));
            assert!(clause.matches(&[Id::raw("b")]));
            assert!(clause.matches(&[Id::raw("a"), Id::raw("b")]));
            assert!(clause.matches(&[Id::raw("a"), Id::raw("c")]));
            assert!(clause.matches(&[Id::raw("d"), Id::raw("b")]));
        }

        #[test]
        fn or_works_with_not() {
            let clause = PartialInheritedClauseConfig {
                or: Some(OneOrMany::Many(vec![Id::raw("a"), Id::raw("b")])),
                not: Some(OneOrMany::Many(vec![Id::raw("c")])),
                ..Default::default()
            };

            assert!(clause.matches(&[Id::raw("a"), Id::raw("b")]));
            assert!(clause.matches(&[Id::raw("a")]));
            assert!(clause.matches(&[Id::raw("b")]));

            assert!(!clause.matches(&[Id::raw("a"), Id::raw("b"), Id::raw("c")]));
            assert!(!clause.matches(&[Id::raw("a"), Id::raw("c")]));
            assert!(!clause.matches(&[Id::raw("b"), Id::raw("c")]));
        }

        #[test]
        fn not_matches_one() {
            let clause = PartialInheritedClauseConfig {
                not: Some(OneOrMany::One(Id::raw("a"))),
                ..Default::default()
            };

            assert!(clause.matches(&[Id::raw("b")]));
            assert!(clause.matches(&[Id::raw("c"), Id::raw("d")]));
        }

        #[test]
        fn not_doesnt_match_one() {
            let clause = PartialInheritedClauseConfig {
                not: Some(OneOrMany::One(Id::raw("a"))),
                ..Default::default()
            };

            assert!(!clause.matches(&[Id::raw("a")]));
            assert!(!clause.matches(&[Id::raw("a"), Id::raw("b")]));
        }

        #[test]
        fn not_matches_many() {
            let clause = PartialInheritedClauseConfig {
                not: Some(OneOrMany::Many(vec![Id::raw("a"), Id::raw("b")])),
                ..Default::default()
            };

            assert!(clause.matches(&[Id::raw("c")]));
            assert!(clause.matches(&[Id::raw("d")]));
            assert!(clause.matches(&[Id::raw("c"), Id::raw("d")]));
        }

        #[test]
        fn not_doesnt_match_many() {
            let clause = PartialInheritedClauseConfig {
                not: Some(OneOrMany::Many(vec![Id::raw("a"), Id::raw("b")])),
                ..Default::default()
            };

            assert!(!clause.matches(&[Id::raw("a")]));
            assert!(!clause.matches(&[Id::raw("b")]));
            assert!(!clause.matches(&[Id::raw("a"), Id::raw("b")]));
            assert!(!clause.matches(&[Id::raw("a"), Id::raw("c")]));
            assert!(!clause.matches(&[Id::raw("b"), Id::raw("d")]));
        }

        #[test]
        fn can_define_all_together() {
            let clause = PartialInheritedClauseConfig {
                and: Some(OneOrMany::Many(vec![Id::raw("a"), Id::raw("b")])),
                or: Some(OneOrMany::Many(vec![Id::raw("c"), Id::raw("d")])),
                not: Some(OneOrMany::One(Id::raw("e"))),
            };

            assert!(clause.matches(&[Id::raw("a"), Id::raw("b"), Id::raw("c")]));
            assert!(clause.matches(&[Id::raw("a"), Id::raw("b"), Id::raw("d")]));
            assert!(clause.matches(&[Id::raw("a"), Id::raw("b"), Id::raw("c"), Id::raw("d")]));

            assert!(!clause.matches(&[Id::raw("a")]));
            assert!(!clause.matches(&[Id::raw("b")]));
            assert!(!clause.matches(&[Id::raw("a"), Id::raw("b")]));
            assert!(!clause.matches(&[Id::raw("c")]));
            assert!(!clause.matches(&[Id::raw("d")]));
            assert!(!clause.matches(&[Id::raw("c"), Id::raw("d")]));
            assert!(!clause.matches(&[Id::raw("a"), Id::raw("b"), Id::raw("c"), Id::raw("e")]));
            assert!(!clause.matches(&[Id::raw("a"), Id::raw("b"), Id::raw("d"), Id::raw("e")]));
            assert!(!clause.matches(&[
                Id::raw("a"),
                Id::raw("b"),
                Id::raw("c"),
                Id::raw("d"),
                Id::raw("e")
            ]));
        }
    }

    mod conditions {
        use super::*;

        #[test]
        fn matches_one() {
            let con = PartialInheritedConditionConfig::One(Id::raw("a"));

            assert!(con.matches(&[Id::raw("a")]));
            assert!(con.matches(&[Id::raw("a"), Id::raw("b")]));
            assert!(!con.matches(&[Id::raw("b")]));
            assert!(!con.matches(&[Id::raw("c")]));
            assert!(!con.matches(&[]));
        }

        #[test]
        fn matches_many() {
            let con = PartialInheritedConditionConfig::Many(vec![Id::raw("a"), Id::raw("b")]);

            assert!(con.matches(&[Id::raw("a")]));
            assert!(con.matches(&[Id::raw("b")]));
            assert!(con.matches(&[Id::raw("a"), Id::raw("b")]));
            assert!(!con.matches(&[Id::raw("c")]));
            assert!(!con.matches(&[]));
        }

        #[test]
        fn matches_clause() {
            let con = PartialInheritedConditionConfig::Clause(PartialInheritedClauseConfig {
                or: Some(OneOrMany::Many(vec![Id::raw("a"), Id::raw("b")])),
                ..Default::default()
            });

            assert!(con.matches(&[Id::raw("a")]));
            assert!(con.matches(&[Id::raw("b")]));
            assert!(con.matches(&[Id::raw("a"), Id::raw("b")]));
            assert!(!con.matches(&[Id::raw("c")]));
            assert!(!con.matches(&[]));
        }
    }

    #[test]
    fn matches_files() {
        let sandbox = create_empty_sandbox();

        let config = PartialInheritedByConfig {
            files: Some(OneOrMany::One(FilePath::parse("file.txt").unwrap())),
            ..Default::default()
        };

        assert!(!config.matches(&InheritFor::default().root(sandbox.path())));

        sandbox.create_file("file.txt", "");

        assert!(config.matches(&InheritFor::default().root(sandbox.path())));
    }

    #[test]
    fn matches_languages() {
        let config = PartialInheritedByConfig {
            languages: Some(OneOrMany::Many(vec![
                LanguageType::JavaScript,
                LanguageType::TypeScript,
            ])),
            ..Default::default()
        };

        assert!(!config.matches(&InheritFor::default().language(&LanguageType::Unknown)));
        assert!(!config.matches(&InheritFor::default().language(&LanguageType::Ruby)));
        assert!(config.matches(&InheritFor::default().language(&LanguageType::JavaScript)));
        assert!(config.matches(&InheritFor::default().language(&LanguageType::TypeScript)));
    }

    #[test]
    fn matches_layers() {
        let config = PartialInheritedByConfig {
            layers: Some(OneOrMany::Many(vec![
                LayerType::Application,
                LayerType::Library,
            ])),
            ..Default::default()
        };

        assert!(!config.matches(&InheritFor::default().layer(&LayerType::Unknown)));
        assert!(!config.matches(&InheritFor::default().layer(&LayerType::Tool)));
        assert!(config.matches(&InheritFor::default().layer(&LayerType::Application)));
        assert!(config.matches(&InheritFor::default().layer(&LayerType::Library)));
    }

    #[test]
    fn matches_stacks() {
        let config = PartialInheritedByConfig {
            stacks: Some(OneOrMany::Many(vec![
                StackType::Frontend,
                StackType::Backend,
            ])),
            ..Default::default()
        };

        assert!(!config.matches(&InheritFor::default().stack(&StackType::Unknown)));
        assert!(!config.matches(&InheritFor::default().stack(&StackType::Systems)));
        assert!(config.matches(&InheritFor::default().stack(&StackType::Frontend)));
        assert!(config.matches(&InheritFor::default().stack(&StackType::Backend)));
    }

    #[test]
    fn matches_tags() {
        let config = PartialInheritedByConfig {
            tags: Some(PartialInheritedConditionConfig::Many(vec![
                Id::raw("a"),
                Id::raw("b"),
            ])),
            ..Default::default()
        };

        assert!(!config.matches(&InheritFor::default().tags(&[])));
        assert!(!config.matches(&InheritFor::default().tags(&[Id::raw("c")])));
        assert!(config.matches(&InheritFor::default().tags(&[Id::raw("a")])));
        assert!(config.matches(&InheritFor::default().tags(&[Id::raw("b")])));
    }

    #[test]
    fn matches_toolchains() {
        let config = PartialInheritedByConfig {
            toolchains: Some(PartialInheritedConditionConfig::Many(vec![
                Id::raw("a"),
                Id::raw("b"),
            ])),
            ..Default::default()
        };

        assert!(!config.matches(&InheritFor::default().toolchains(&[])));
        assert!(!config.matches(&InheritFor::default().toolchains(&[Id::raw("c")])));
        assert!(config.matches(&InheritFor::default().toolchains(&[Id::raw("a")])));
        assert!(config.matches(&InheritFor::default().toolchains(&[Id::raw("b")])));
    }

    #[test]
    fn matches_using_all_params() {
        let config = PartialInheritedByConfig {
            layers: Some(OneOrMany::Many(vec![
                LayerType::Application,
                LayerType::Library,
            ])),
            stacks: Some(OneOrMany::Many(vec![
                StackType::Frontend,
                StackType::Backend,
            ])),
            tags: Some(PartialInheritedConditionConfig::Many(vec![
                Id::raw("a"),
                Id::raw("b"),
            ])),
            toolchains: Some(PartialInheritedConditionConfig::Many(vec![
                Id::raw("c"),
                Id::raw("d"),
            ])),
            ..Default::default()
        };

        assert!(
            !config.matches(
                &InheritFor::default()
                    .toolchains(&[Id::raw("z")])
                    .stack(&StackType::Frontend)
                    .layer(&LayerType::Scaffolding)
                    .tags(&[Id::raw("y")])
            )
        );
        assert!(
            !config.matches(
                &InheritFor::default()
                    .toolchains(&[Id::raw("d")])
                    .stack(&StackType::Frontend)
                    .layer(&LayerType::Scaffolding)
                    .tags(&[Id::raw("y")])
            )
        );
        assert!(
            !config.matches(
                &InheritFor::default()
                    .toolchains(&[Id::raw("d")])
                    .stack(&StackType::Frontend)
                    .layer(&LayerType::Scaffolding)
                    .tags(&[Id::raw("a")])
            )
        );
        assert!(
            config.matches(
                &InheritFor::default()
                    .toolchains(&[Id::raw("d")])
                    .stack(&StackType::Frontend)
                    .layer(&LayerType::Application)
                    .tags(&[Id::raw("a")])
            )
        );
    }
}
