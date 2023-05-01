use moon_config::{PlatformType, ProjectLanguage, ProjectType, TaskType};
use moon_query::{build_query, ComparisonOperator, Condition, Criteria, Field, LogicalOperator};
use starbase_utils::string_vec;

mod mql_build {
    use super::*;

    #[test]
    #[should_panic(expected = "EmptyInput")]
    fn errors_if_empty() {
        build_query("").unwrap();
    }

    #[test]
    #[should_panic(expected = "UnknownField(\"key\")")]
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
                        field: Field::Language(vec![ProjectLanguage::JavaScript]),
                        op: ComparisonOperator::Equal,
                    },
                    Condition::Field {
                        field: Field::Language(vec![ProjectLanguage::TypeScript]),
                        op: ComparisonOperator::NotEqual,
                    }
                ],
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
                        field: Field::Language(vec![ProjectLanguage::JavaScript]),
                        op: ComparisonOperator::Equal,
                    },
                    Condition::Field {
                        field: Field::Language(vec![ProjectLanguage::TypeScript]),
                        op: ComparisonOperator::NotEqual,
                    }
                ],
            }
        );
    }

    #[test]
    #[should_panic(expected = "LogicalOperatorMismatch")]
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
                            field: Field::Language(vec![ProjectLanguage::JavaScript]),
                            op: ComparisonOperator::Equal,
                        },
                        Condition::Criteria {
                            criteria: Criteria {
                                op: LogicalOperator::Or,
                                conditions: vec![
                                    Condition::Field {
                                        field: Field::Task(string_vec!["foo"]),
                                        op: ComparisonOperator::Equal,
                                    },
                                    Condition::Field {
                                        field: Field::Task(string_vec!["bar"]),
                                        op: ComparisonOperator::NotEqual,
                                    },
                                    Condition::Field {
                                        field: Field::Task(string_vec!["baz"]),
                                        op: ComparisonOperator::Like,
                                    }
                                ],
                            }
                        }
                    ],
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
                        field: Field::Language(vec![ProjectLanguage::JavaScript]),
                        op: ComparisonOperator::Equal,
                    }, Condition::Criteria { criteria: Criteria {
                        op: LogicalOperator::Or,
                        conditions: vec![
                            Condition::Field {
                                field: Field::Task(string_vec!["foo"]),
                                op: ComparisonOperator::Equal,
                            },
                            Condition::Field {
                                field: Field::Task(string_vec!["bar"]),
                                op: ComparisonOperator::NotEqual,
                            },
                        ],
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
                    } }],
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
                            field: Field::Language(vec![ProjectLanguage::JavaScript]),
                            op: ComparisonOperator::Equal,
                        },
                        Condition::Criteria {
                            criteria: Criteria {
                                op: LogicalOperator::Or,
                                conditions: vec![
                                    Condition::Field {
                                        field: Field::Task(string_vec!["foo"]),
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
                                        }
                                    }
                                ],
                            }
                        }
                    ],
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
        //                 field: Field::Language(vec![ProjectLanguage::JavaScript]),
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
        //                             field: Field::Task(string_vec!["lint*"]),
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
        //                             field: Field::Task(string_vec!["build*"]),
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
                        field: Field::Language(vec![ProjectLanguage::JavaScript]),
                        op: ComparisonOperator::Equal,
                    }],
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
                        field: Field::Language(vec![ProjectLanguage::Other("other".into())]),
                        op: ComparisonOperator::NotEqual,
                    }],
                }
            );
        }

        #[test]
        #[should_panic(expected = "UnsupportedLikeOperator(\"language\")")]
        fn errors_for_like() {
            build_query("language~javascript").unwrap();
        }

        #[test]
        #[should_panic(expected = "UnsupportedLikeOperator(\"language\")")]
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
                        field: Field::Project(string_vec!["foo"]),
                        op: ComparisonOperator::NotEqual,
                    }],
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
                        field: Field::Project(string_vec!["foo*"]),
                        op: ComparisonOperator::Like,
                    }],
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
                        field: Field::ProjectAlias(string_vec!["foo"]),
                        op: ComparisonOperator::Equal,
                    }],
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
                        field: Field::ProjectAlias(string_vec!["foo*"]),
                        op: ComparisonOperator::NotLike,
                    }],
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
                        field: Field::ProjectAlias(string_vec!["@scope/*"]),
                        op: ComparisonOperator::Like,
                    }],
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
                        field: Field::ProjectSource(string_vec!["packages/foo"]),
                        op: ComparisonOperator::NotEqual,
                    }],
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
                        field: Field::ProjectSource(string_vec!["packages/*"]),
                        op: ComparisonOperator::NotLike,
                    }],
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
                        field: Field::ProjectType(vec![ProjectType::Library]),
                        op: ComparisonOperator::Equal,
                    }],
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
                        field: Field::ProjectType(vec![ProjectType::Tool, ProjectType::Library]),
                        op: ComparisonOperator::NotEqual,
                    }],
                }
            );
        }

        #[test]
        #[should_panic(expected = "UnknownFieldValue(\"projectType\", \"app\")")]
        fn invalid_value() {
            build_query("projectType=app").unwrap();
        }

        #[test]
        #[should_panic(expected = "UnsupportedLikeOperator(\"projectType\")")]
        fn errors_for_like() {
            build_query("projectType~library").unwrap();
        }

        #[test]
        #[should_panic(expected = "UnsupportedLikeOperator(\"projectType\")")]
        fn errors_for_not_like() {
            build_query("projectType!~tool").unwrap();
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
                        field: Field::Tag(string_vec!["lib"]),
                        op: ComparisonOperator::Equal,
                    }],
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
                        field: Field::Tag(string_vec!["foo", "bar"]),
                        op: ComparisonOperator::NotEqual,
                    }],
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
                        field: Field::Tag(string_vec!["app-*"]),
                        op: ComparisonOperator::Like,
                    }],
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
                        field: Field::Task(string_vec!["foo"]),
                        op: ComparisonOperator::NotEqual,
                    }],
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
                        field: Field::Task(string_vec!["foo*"]),
                        op: ComparisonOperator::Like,
                    }],
                }
            );
        }
    }

    mod task_platform {
        use super::*;

        #[test]
        fn valid_value() {
            assert_eq!(
                build_query("taskPlatform=node").unwrap(),
                Criteria {
                    op: LogicalOperator::And,
                    conditions: vec![Condition::Field {
                        field: Field::TaskPlatform(vec![PlatformType::Node]),
                        op: ComparisonOperator::Equal,
                    }],
                }
            );
        }

        #[test]
        fn valid_value_list() {
            assert_eq!(
                build_query("taskPlatform!=[node, system]").unwrap(),
                Criteria {
                    op: LogicalOperator::And,
                    conditions: vec![Condition::Field {
                        field: Field::TaskPlatform(vec![PlatformType::Node, PlatformType::System]),
                        op: ComparisonOperator::NotEqual,
                    }],
                }
            );
        }

        #[test]
        #[should_panic(expected = "UnknownFieldValue(\"taskPlatform\", \"kotlin\")")]
        fn invalid_value() {
            build_query("taskPlatform=kotlin").unwrap();
        }

        #[test]
        #[should_panic(expected = "UnsupportedLikeOperator(\"taskPlatform\")")]
        fn errors_for_like() {
            build_query("taskPlatform~node").unwrap();
        }

        #[test]
        #[should_panic(expected = "UnsupportedLikeOperator(\"taskPlatform\")")]
        fn errors_for_not_like() {
            build_query("taskPlatform!~node").unwrap();
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
                }
            );
        }

        #[test]
        #[should_panic(expected = "UnknownFieldValue(\"taskType\", \"kotlin\")")]
        fn invalid_value() {
            build_query("taskType=kotlin").unwrap();
        }

        #[test]
        #[should_panic(expected = "UnsupportedLikeOperator(\"taskType\")")]
        fn errors_for_like() {
            build_query("taskType~node").unwrap();
        }

        #[test]
        #[should_panic(expected = "UnsupportedLikeOperator(\"taskType\")")]
        fn errors_for_not_like() {
            build_query("taskType!~node").unwrap();
        }
    }
}
