use moon_common::Id;
use moon_config::{
    FilePath, LayerType, OneOrMany, PartialInheritedByConfig, PartialInheritedClauseConfig,
    PartialInheritedConditionConfig, PortablePath, StackType,
};
use starbase_sandbox::create_empty_sandbox;
use std::path::Path;

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

        assert!(!config.matches(
            sandbox.path(),
            &[],
            &StackType::Unknown,
            &LayerType::Unknown,
            &[]
        ));

        sandbox.create_file("file.txt", "");

        assert!(config.matches(
            sandbox.path(),
            &[],
            &StackType::Unknown,
            &LayerType::Unknown,
            &[]
        ));
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

        assert!(!config.matches(
            Path::new(""),
            &[],
            &StackType::Unknown,
            &LayerType::Unknown,
            &[]
        ));
        assert!(!config.matches(
            Path::new(""),
            &[],
            &StackType::Unknown,
            &LayerType::Tool,
            &[]
        ));
        assert!(config.matches(
            Path::new(""),
            &[],
            &StackType::Unknown,
            &LayerType::Application,
            &[]
        ));
        assert!(config.matches(
            Path::new(""),
            &[],
            &StackType::Unknown,
            &LayerType::Library,
            &[]
        ));
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

        assert!(!config.matches(
            Path::new(""),
            &[],
            &StackType::Unknown,
            &LayerType::Unknown,
            &[]
        ));
        assert!(!config.matches(
            Path::new(""),
            &[],
            &StackType::Systems,
            &LayerType::Unknown,
            &[]
        ));
        assert!(config.matches(
            Path::new(""),
            &[],
            &StackType::Frontend,
            &LayerType::Unknown,
            &[]
        ));
        assert!(config.matches(
            Path::new(""),
            &[],
            &StackType::Backend,
            &LayerType::Unknown,
            &[]
        ));
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

        // matches because empty arrays are skipped
        assert!(config.matches(
            Path::new(""),
            &[],
            &StackType::Unknown,
            &LayerType::Unknown,
            &[]
        ));
        assert!(!config.matches(
            Path::new(""),
            &[],
            &StackType::Unknown,
            &LayerType::Unknown,
            &[Id::raw("c")]
        ));
        assert!(config.matches(
            Path::new(""),
            &[],
            &StackType::Unknown,
            &LayerType::Unknown,
            &[Id::raw("a")]
        ));
        assert!(config.matches(
            Path::new(""),
            &[],
            &StackType::Unknown,
            &LayerType::Unknown,
            &[Id::raw("b")]
        ));
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

        // matches because empty arrays are skipped
        assert!(config.matches(
            Path::new(""),
            &[],
            &StackType::Unknown,
            &LayerType::Unknown,
            &[]
        ));
        assert!(!config.matches(
            Path::new(""),
            &[Id::raw("c")],
            &StackType::Unknown,
            &LayerType::Unknown,
            &[]
        ));
        assert!(config.matches(
            Path::new(""),
            &[Id::raw("a")],
            &StackType::Unknown,
            &LayerType::Unknown,
            &[]
        ));
        assert!(config.matches(
            Path::new(""),
            &[Id::raw("b")],
            &StackType::Unknown,
            &LayerType::Unknown,
            &[]
        ));
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

        assert!(!config.matches(
            Path::new(""),
            &[],
            &StackType::Unknown,
            &LayerType::Unknown,
            &[]
        ));
        assert!(!config.matches(
            Path::new(""),
            &[Id::raw("z")],
            &StackType::Frontend,
            &LayerType::Scaffolding,
            &[Id::raw("y")]
        ));
        assert!(!config.matches(
            Path::new(""),
            &[Id::raw("d")],
            &StackType::Frontend,
            &LayerType::Scaffolding,
            &[Id::raw("y")]
        ));
        assert!(!config.matches(
            Path::new(""),
            &[Id::raw("d")],
            &StackType::Frontend,
            &LayerType::Scaffolding,
            &[Id::raw("a")]
        ));
        assert!(config.matches(
            Path::new(""),
            &[Id::raw("d")],
            &StackType::Frontend,
            &LayerType::Application,
            &[Id::raw("a")]
        ));
    }
}
