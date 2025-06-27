use moon_config::{LanguageType, LayerType, StackType, TaskType};
use moon_query::{
    ComparisonOperator, Condition, Criteria, Field, FieldValues, LogicalOperator, build_query,
};
use std::borrow::Cow;

fn value_list<I: IntoIterator<Item = V>, V: AsRef<str>>(list: I) -> FieldValues<'static> {
    list.into_iter()
        .map(|s| Cow::Owned(s.as_ref().to_owned()))
        .collect()
}

mod mql_build {
    use super::*;

    #[test]
    #[should_panic(expected = "Encountered an empty query.")]
    fn errors_if_empty() {
        build_query("").unwrap();
    }

    #[test]
    #[should_panic(expected = "Unknown query field key.")]
    fn errors_unknown_field() {
        build_query("key=value").unwrap();
    }

    #[test]
    fn handles_and() {
        assert_eq!(
            build_query("language=javascript AND language!=typescript").unwrap(),
            Criteria {
                op: LogicalOperator::And,
                conditions: vec![
                    Condition::Field {
                        field: Field::Language(vec![LanguageType::JavaScript]),
                        op: ComparisonOperator::Equal,
                    },
                    Condition::Field {
                        field: Field::Language(vec![LanguageType::TypeScript]),
                        op: ComparisonOperator::NotEqual,
                    }
                ],
                input: Some("language=javascript AND language!=typescript".into())
            },
        );
    }

    #[test]
    fn handles_or() {
        assert_eq!(
            build_query("language=javascript || language!=typescript").unwrap(),
            Criteria {
                op: LogicalOperator::Or,
                conditions: vec![
                    Condition::Field {
                        field: Field::Language(vec![LanguageType::JavaScript]),
                        op: ComparisonOperator::Equal,
                    },
                    Condition::Field {
                        field: Field::Language(vec![LanguageType::TypeScript]),
                        op: ComparisonOperator::NotEqual,
                    }
                ],
                input: Some("language=javascript || language!=typescript".into())
            }
        );
    }

    #[test]
    #[should_panic(expected = "Cannot use both AND (&&) and OR (||) logical operators")]
    fn errors_when_mixing_ops() {
        build_query("language=javascript || language!=typescript && language=ruby").unwrap();
    }

    mod nested {
        use super::*;

        #[test]
        fn depth_1() {
            assert_eq!(
                build_query("language=javascript AND (task=foo || task!=bar OR task~baz)").unwrap(),
                Criteria {
                    op: LogicalOperator::And,
                    conditions: vec![
                        Condition::Field {
                            field: Field::Language(vec![LanguageType::JavaScript]),
                            op: ComparisonOperator::Equal,
                        },
                        Condition::Criteria {
                            criteria: Criteria {
                                op: LogicalOperator::Or,
                                conditions: vec![
                                    Condition::Field {
                                        field: Field::Task(value_list(["foo"])),
                                        op: ComparisonOperator::Equal,
                                    },
                                    Condition::Field {
                                        field: Field::Task(value_list(["bar"])),
                                        op: ComparisonOperator::NotEqual,
                                    },
                                    Condition::Field {
                                        field: Field::Task(value_list(["baz"])),
                                        op: ComparisonOperator::Like,
                                    }
                                ],
                                input: None,
                            }
                        }
                    ],
                    input: Some(
                        "language=javascript AND (task=foo || task!=bar OR task~baz)".into()
                    )
                }
            );
        }

        #[test]
        fn depth_1_siblings() {
            assert_eq!(
                build_query("language=javascript AND (task=foo || task!=bar) && (taskType=build AND taskType=run)").unwrap(),
                Criteria {
                    op: LogicalOperator::And,
                    conditions: vec![Condition::Field {
                        field: Field::Language(vec![LanguageType::JavaScript]),
                        op: ComparisonOperator::Equal,
                    }, Condition::Criteria { criteria: Criteria {
                        op: LogicalOperator::Or,
                        conditions: vec![
                            Condition::Field {
                                field: Field::Task(value_list(["foo"])),
                                op: ComparisonOperator::Equal,
                            },
                            Condition::Field {
                                field: Field::Task(value_list(["bar"])),
                                op: ComparisonOperator::NotEqual,
                            },
                        ],
                        input: None,
                    } }, Condition::Criteria { criteria: Criteria {
                        op: LogicalOperator::And,
                        conditions: vec![
                            Condition::Field {
                                field: Field::TaskType(vec![TaskType::Build]),
                                op: ComparisonOperator::Equal,
                            },
                            Condition::Field {
                                field: Field::TaskType(vec![TaskType::Run]),
                                op: ComparisonOperator::Equal,
                            },
                        ],
                        input: None
                    } }],
                    input: Some("language=javascript AND (task=foo || task!=bar) && (taskType=build AND taskType=run)".into())
                }
            );
        }

        #[test]
        fn depth_2() {
            assert_eq!(
                build_query(
                    "language=javascript AND (task=foo || (taskType=build AND taskType=run))"
                )
                .unwrap(),
                Criteria {
                    op: LogicalOperator::And,
                    conditions: vec![
                        Condition::Field {
                            field: Field::Language(vec![LanguageType::JavaScript]),
                            op: ComparisonOperator::Equal,
                        },
                        Condition::Criteria {
                            criteria: Criteria {
                                op: LogicalOperator::Or,
                                conditions: vec![
                                    Condition::Field {
                                        field: Field::Task(value_list(["foo"])),
                                        op: ComparisonOperator::Equal,
                                    },
                                    Condition::Criteria {
                                        criteria: Criteria {
                                            op: LogicalOperator::And,
                                            conditions: vec![
                                                Condition::Field {
                                                    field: Field::TaskType(vec![TaskType::Build]),
                                                    op: ComparisonOperator::Equal,
                                                },
                                                Condition::Field {
                                                    field: Field::TaskType(vec![TaskType::Run]),
                                                    op: ComparisonOperator::Equal,
                                                },
                                            ],
                                            input: None,
                                        }
                                    }
                                ],
                                input: None,
                            }
                        }
                    ],
                    input: Some(
                        "language=javascript AND (task=foo || (taskType=build AND taskType=run))"
                            .into()
                    )
                }
            );
        }

        // #[test]
        // fn depth_2_siblings() {
        //     assert_eq!(
        //         build("language=javascript && ((taskType=test && task~lint*) || (taskType=build && task~build*))")
        //             .unwrap(),
        //         QueryCriteria {
        //             op: Some(LogicalOperator::And),
        //             fields: vec![QueryField {
        //                 field: Field::Language(vec![LanguageType::JavaScript]),
        //                 op: ComparisonOperator::Equal,
        //             },],
        //             criteria: vec![QueryCriteria {
        //                 op: Some(LogicalOperator::Or),
        //                 fields: vec![],
        //                 criteria: vec![QueryCriteria {
        //                     op: Some(LogicalOperator::And),
        //                     fields: vec![
        //                         QueryField {
        //                             field: Field::TaskType(vec![TaskType::Test]),
        //                             op: ComparisonOperator::Equal,
        //                         },
        //                         QueryField {
        //                             field: Field::Task(value_list(["lint*"]),
        //                             op: ComparisonOperator::Like,
        //                         },
        //                     ],
        //                     criteria: vec![],
        //                 }, QueryCriteria {
        //                     op: Some(LogicalOperator::And),
        //                     fields: vec![
        //                         QueryField {
        //                             field: Field::TaskType(vec![TaskType::Build]),
        //                             op: ComparisonOperator::Equal,
        //                         },
        //                         QueryField {
        //                             field: Field::Task(value_list(["build*"]),
        //                             op: ComparisonOperator::Like,
        //                         },
        //                     ],
        //                     criteria: vec![],
        //                 }],
        //             }],
        //         }
        //     );
        // }
    }

    mod language {
        use super::*;

        #[test]
        fn valid_value() {
            assert_eq!(
                build_query("language=javascript").unwrap(),
                Criteria {
                    op: LogicalOperator::And,
                    conditions: vec![Condition::Field {
                        field: Field::Language(vec![LanguageType::JavaScript]),
                        op: ComparisonOperator::Equal,
                    }],
                    input: Some("language=javascript".into())
                }
            );
        }

        #[test]
        fn other_value() {
            assert_eq!(
                build_query("language!=other").unwrap(),
                Criteria {
                    op: LogicalOperator::And,
                    conditions: vec![Condition::Field {
                        field: Field::Language(vec![LanguageType::other("other").unwrap()]),
                        op: ComparisonOperator::NotEqual,
                    }],
                    input: Some("language!=other".into())
                }
            );
        }

        #[test]
        #[should_panic(
            expected = "Like operators (~ and !~) are not supported for field language."
        )]
        fn errors_for_like() {
            build_query("language~javascript").unwrap();
        }

        #[test]
        #[should_panic(
            expected = "Like operators (~ and !~) are not supported for field language."
        )]
        fn errors_for_not_like() {
            build_query("language!~javascript").unwrap();
        }
    }

    mod project {
        use super::*;

        #[test]
        fn name_eq() {
            assert_eq!(
                build_query("project!=foo").unwrap(),
                Criteria {
                    op: LogicalOperator::And,
                    conditions: vec![Condition::Field {
                        field: Field::Project(value_list(["foo"])),
                        op: ComparisonOperator::NotEqual,
                    }],
                    input: Some("project!=foo".into())
                }
            );
        }

        #[test]
        fn name_like() {
            assert_eq!(
                build_query("project~foo*").unwrap(),
                Criteria {
                    op: LogicalOperator::And,
                    conditions: vec![Condition::Field {
                        field: Field::Project(value_list(["foo*"])),
                        op: ComparisonOperator::Like,
                    }],
                    input: Some("project~foo*".into())
                }
            );
        }
    }

    mod project_alias {
        use super::*;

        #[test]
        fn alias_eq() {
            assert_eq!(
                build_query("projectAlias=foo").unwrap(),
                Criteria {
                    op: LogicalOperator::And,
                    conditions: vec![Condition::Field {
                        field: Field::ProjectAlias(value_list(["foo"])),
                        op: ComparisonOperator::Equal,
                    }],
                    input: Some("projectAlias=foo".into())
                }
            );
        }

        #[test]
        fn alias_like() {
            assert_eq!(
                build_query("projectAlias!~foo*").unwrap(),
                Criteria {
                    op: LogicalOperator::And,
                    conditions: vec![Condition::Field {
                        field: Field::ProjectAlias(value_list(["foo*"])),
                        op: ComparisonOperator::NotLike,
                    }],
                    input: Some("projectAlias!~foo*".into())
                }
            );
        }

        #[test]
        fn alias_like_scope() {
            assert_eq!(
                build_query("projectAlias~@scope/*").unwrap(),
                Criteria {
                    op: LogicalOperator::And,
                    conditions: vec![Condition::Field {
                        field: Field::ProjectAlias(value_list(["@scope/*"])),
                        op: ComparisonOperator::Like,
                    }],
                    input: Some("projectAlias~@scope/*".into())
                }
            );
        }
    }

    mod project_source {
        use super::*;

        #[test]
        fn source_eq() {
            assert_eq!(
                build_query("projectSource!=packages/foo").unwrap(),
                Criteria {
                    op: LogicalOperator::And,
                    conditions: vec![Condition::Field {
                        field: Field::ProjectSource(value_list(["packages/foo"])),
                        op: ComparisonOperator::NotEqual,
                    }],
                    input: Some("projectSource!=packages/foo".into())
                }
            );
        }

        #[test]
        fn source_like() {
            assert_eq!(
                build_query("projectSource!~packages/*").unwrap(),
                Criteria {
                    op: LogicalOperator::And,
                    conditions: vec![Condition::Field {
                        field: Field::ProjectSource(value_list(["packages/*"])),
                        op: ComparisonOperator::NotLike,
                    }],
                    input: Some("projectSource!~packages/*".into())
                }
            );
        }
    }

    mod project_type {
        use super::*;

        #[test]
        fn valid_value() {
            assert_eq!(
                build_query("projectType=library").unwrap(),
                Criteria {
                    op: LogicalOperator::And,
                    conditions: vec![Condition::Field {
                        field: Field::ProjectType(vec![LayerType::Library]),
                        op: ComparisonOperator::Equal,
                    }],
                    input: Some("projectType=library".into())
                }
            );
        }

        #[test]
        fn valid_value_list() {
            assert_eq!(
                build_query("projectType!=[tool, library]").unwrap(),
                Criteria {
                    op: LogicalOperator::And,
                    conditions: vec![Condition::Field {
                        field: Field::ProjectType(vec![LayerType::Tool, LayerType::Library]),
                        op: ComparisonOperator::NotEqual,
                    }],
                    input: Some("projectType!=[tool, library]".into())
                }
            );
        }

        #[test]
        #[should_panic(expected = "Unknown query value app for field projectType.")]
        fn invalid_value() {
            build_query("projectType=app").unwrap();
        }

        #[test]
        #[should_panic(
            expected = "Like operators (~ and !~) are not supported for field projectType."
        )]
        fn errors_for_like() {
            build_query("projectType~library").unwrap();
        }

        #[test]
        #[should_panic(
            expected = "Like operators (~ and !~) are not supported for field projectType."
        )]
        fn errors_for_not_like() {
            build_query("projectType!~tool").unwrap();
        }
    }

    mod project_stack {
        use super::*;

        #[test]
        fn valid_value() {
            assert_eq!(
                build_query("projectStack=frontend").unwrap(),
                Criteria {
                    op: LogicalOperator::And,
                    conditions: vec![Condition::Field {
                        field: Field::ProjectStack(vec![StackType::Frontend]),
                        op: ComparisonOperator::Equal,
                    }],
                    input: Some("projectStack=frontend".into())
                }
            );
        }

        #[test]
        fn valid_value_list() {
            assert_eq!(
                build_query("projectStack!=[frontend, backend]").unwrap(),
                Criteria {
                    op: LogicalOperator::And,
                    conditions: vec![Condition::Field {
                        field: Field::ProjectStack(vec![StackType::Frontend, StackType::Backend]),
                        op: ComparisonOperator::NotEqual,
                    }],
                    input: Some("projectStack!=[frontend, backend]".into())
                }
            );
        }

        #[test]
        #[should_panic(expected = "Unknown query value midend for field projectStack.")]
        fn invalid_value() {
            build_query("projectStack=midend").unwrap();
        }

        #[test]
        #[should_panic(
            expected = "Like operators (~ and !~) are not supported for field projectStack."
        )]
        fn errors_for_like() {
            build_query("projectStack~systems").unwrap();
        }

        #[test]
        #[should_panic(
            expected = "Like operators (~ and !~) are not supported for field projectStack."
        )]
        fn errors_for_not_like() {
            build_query("projectStack!~systems").unwrap();
        }
    }

    mod tag {
        use super::*;

        #[test]
        fn tag_eq() {
            assert_eq!(
                build_query("tag=lib").unwrap(),
                Criteria {
                    op: LogicalOperator::And,
                    conditions: vec![Condition::Field {
                        field: Field::Tag(value_list(["lib"])),
                        op: ComparisonOperator::Equal,
                    }],
                    input: Some("tag=lib".into())
                }
            );
        }

        #[test]
        fn tag_neq_list() {
            assert_eq!(
                build_query("tag!=[foo,bar]").unwrap(),
                Criteria {
                    op: LogicalOperator::And,
                    conditions: vec![Condition::Field {
                        field: Field::Tag(value_list(["foo", "bar"])),
                        op: ComparisonOperator::NotEqual,
                    }],
                    input: Some("tag!=[foo,bar]".into())
                }
            );
        }

        #[test]
        fn tag_like() {
            assert_eq!(
                build_query("tag~app-*").unwrap(),
                Criteria {
                    op: LogicalOperator::And,
                    conditions: vec![Condition::Field {
                        field: Field::Tag(value_list(["app-*"])),
                        op: ComparisonOperator::Like,
                    }],
                    input: Some("tag~app-*".into())
                }
            );
        }
    }

    mod task {
        use super::*;

        #[test]
        fn task_eq() {
            assert_eq!(
                build_query("task!=foo").unwrap(),
                Criteria {
                    op: LogicalOperator::And,
                    conditions: vec![Condition::Field {
                        field: Field::Task(value_list(["foo"])),
                        op: ComparisonOperator::NotEqual,
                    }],
                    input: Some("task!=foo".into())
                }
            );
        }

        #[test]
        fn task_like() {
            assert_eq!(
                build_query("task~foo*").unwrap(),
                Criteria {
                    op: LogicalOperator::And,
                    conditions: vec![Condition::Field {
                        field: Field::Task(value_list(["foo*"])),
                        op: ComparisonOperator::Like,
                    }],
                    input: Some("task~foo*".into())
                }
            );
        }
    }

    mod task_toolchain {
        use super::*;

        #[test]
        fn valid_value() {
            assert_eq!(
                build_query("taskToolchain=node").unwrap(),
                Criteria {
                    op: LogicalOperator::And,
                    conditions: vec![Condition::Field {
                        field: Field::TaskToolchain(value_list(["node"])),
                        op: ComparisonOperator::Equal,
                    }],
                    input: Some("taskToolchain=node".into())
                }
            );
        }

        #[test]
        fn valid_value_list() {
            assert_eq!(
                build_query("taskToolchain!=[node, system]").unwrap(),
                Criteria {
                    op: LogicalOperator::And,
                    conditions: vec![Condition::Field {
                        field: Field::TaskToolchain(value_list(["node", "system"])),
                        op: ComparisonOperator::NotEqual,
                    }],
                    input: Some("taskToolchain!=[node, system]".into())
                }
            );
        }
    }

    mod task_type {
        use super::*;

        #[test]
        fn valid_value() {
            assert_eq!(
                build_query("taskType=build").unwrap(),
                Criteria {
                    op: LogicalOperator::And,
                    conditions: vec![Condition::Field {
                        field: Field::TaskType(vec![TaskType::Build]),
                        op: ComparisonOperator::Equal,
                    }],
                    input: Some("taskType=build".into())
                }
            );
        }

        #[test]
        #[should_panic(expected = "Unknown query value kotlin for field taskType.")]
        fn invalid_value() {
            build_query("taskType=kotlin").unwrap();
        }

        #[test]
        #[should_panic(
            expected = "Like operators (~ and !~) are not supported for field taskType."
        )]
        fn errors_for_like() {
            build_query("taskType~node").unwrap();
        }

        #[test]
        #[should_panic(
            expected = "Like operators (~ and !~) are not supported for field taskType."
        )]
        fn errors_for_not_like() {
            build_query("taskType!~node").unwrap();
        }
    }
}
