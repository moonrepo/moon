use moon_query::{parse_query, AstNode, ComparisonOperator, LogicalOperator};

mod mql_parse {
    use super::*;

    #[test]
    #[should_panic]
    fn errors_if_empty() {
        parse_query("").unwrap();
    }

    #[test]
    #[should_panic]
    fn errors_no_logic_op() {
        parse_query("k1=v1 k2=v2").unwrap();
    }

    #[test]
    #[should_panic]
    fn errors_invalid_eq() {
        parse_query("k1==v2").unwrap();
    }

    #[test]
    #[should_panic]
    fn errors_invalid_op() {
        parse_query("k1=v1 & k2=v2").unwrap();
    }

    #[test]
    #[should_panic]
    fn errors_double_logic_op() {
        parse_query("k1=v1 && && k2=v2").unwrap();
    }

    #[test]
    fn comp_eq() {
        assert_eq!(
            parse_query("key=value").unwrap(),
            vec![AstNode::Comparison {
                field: "key".into(),
                op: ComparisonOperator::Equal,
                value: vec!["value".into()],
            }],
        );
    }

    #[test]
    fn comp_eq_list() {
        assert_eq!(
            parse_query("key=[v1, v2, v3]").unwrap(),
            vec![AstNode::Comparison {
                field: "key".into(),
                op: ComparisonOperator::Equal,
                value: vec!["v1".into(), "v2".into(), "v3".into()],
            }],
        );
    }

    #[test]
    fn comp_neq() {
        assert_eq!(
            parse_query("key!=value").unwrap(),
            vec![AstNode::Comparison {
                field: "key".into(),
                op: ComparisonOperator::NotEqual,
                value: vec!["value".into()],
            }],
        );
    }

    #[test]
    fn comp_neq_list() {
        assert_eq!(
            parse_query("key!=[v1,v2,v3]").unwrap(),
            vec![AstNode::Comparison {
                field: "key".into(),
                op: ComparisonOperator::NotEqual,
                value: vec!["v1".into(), "v2".into(), "v3".into()],
            }],
        );
    }

    #[test]
    fn comp_like() {
        assert_eq!(
            parse_query("key~value").unwrap(),
            vec![AstNode::Comparison {
                field: "key".into(),
                op: ComparisonOperator::Like,
                value: vec!["value".into()],
            }],
        );
    }

    #[test]
    fn comp_nlike() {
        assert_eq!(
            parse_query("key!~value").unwrap(),
            vec![AstNode::Comparison {
                field: "key".into(),
                op: ComparisonOperator::NotLike,
                value: vec!["value".into()],
            }],
        );
    }

    #[test]
    fn multi_and_comp() {
        assert_eq!(
            parse_query("k1=v1 && k2!=v2 AND k3=[1,2,3]").unwrap(),
            vec![
                AstNode::Comparison {
                    field: "k1".into(),
                    op: ComparisonOperator::Equal,
                    value: vec!["v1".into()],
                },
                AstNode::Op {
                    op: LogicalOperator::And,
                },
                AstNode::Comparison {
                    field: "k2".into(),
                    op: ComparisonOperator::NotEqual,
                    value: vec!["v2".into()],
                },
                AstNode::Op {
                    op: LogicalOperator::And,
                },
                AstNode::Comparison {
                    field: "k3".into(),
                    op: ComparisonOperator::Equal,
                    value: vec!["1".into(), "2".into(), "3".into()],
                }
            ],
        );
    }

    #[test]
    fn multi_or_comp() {
        assert_eq!(
            parse_query("k1=v1 || k2!=v2 OR k3=[1,2,3]").unwrap(),
            vec![
                AstNode::Comparison {
                    field: "k1".into(),
                    op: ComparisonOperator::Equal,
                    value: vec!["v1".into()],
                },
                AstNode::Op {
                    op: LogicalOperator::Or,
                },
                AstNode::Comparison {
                    field: "k2".into(),
                    op: ComparisonOperator::NotEqual,
                    value: vec!["v2".into()],
                },
                AstNode::Op {
                    op: LogicalOperator::Or,
                },
                AstNode::Comparison {
                    field: "k3".into(),
                    op: ComparisonOperator::Equal,
                    value: vec!["1".into(), "2".into(), "3".into()],
                }
            ],
        );
    }

    #[test]
    fn cmp_op_space() {
        assert_eq!(
            parse_query("key =  value").unwrap(),
            vec![AstNode::Comparison {
                field: "key".into(),
                op: ComparisonOperator::Equal,
                value: vec!["value".into()],
            }],
        );
    }

    #[test]
    fn lgc_op_space() {
        assert_eq!(
            parse_query("k1=v1&&      k2!=v2").unwrap(),
            vec![
                AstNode::Comparison {
                    field: "k1".into(),
                    op: ComparisonOperator::Equal,
                    value: vec!["v1".into()],
                },
                AstNode::Op {
                    op: LogicalOperator::And,
                },
                AstNode::Comparison {
                    field: "k2".into(),
                    op: ComparisonOperator::NotEqual,
                    value: vec!["v2".into()],
                }
            ],
        );
    }

    #[test]
    fn group() {
        assert_eq!(
            parse_query("k1=v1 && (k2 != v2 OR k3  =  v3)").unwrap(),
            vec![
                AstNode::Comparison {
                    field: "k1".into(),
                    op: ComparisonOperator::Equal,
                    value: vec!["v1".into()],
                },
                AstNode::Op {
                    op: LogicalOperator::And,
                },
                AstNode::Group {
                    nodes: vec![
                        AstNode::Comparison {
                            field: "k2".into(),
                            op: ComparisonOperator::NotEqual,
                            value: vec!["v2".into()],
                        },
                        AstNode::Op {
                            op: LogicalOperator::Or,
                        },
                        AstNode::Comparison {
                            field: "k3".into(),
                            op: ComparisonOperator::Equal,
                            value: vec!["v3".into()],
                        }
                    ]
                }
            ],
        );
    }

    #[test]
    fn multi_group() {
        assert_eq!(
            parse_query("k1=v1 && (k2!=v2 OR k3=v3) AND (k4=v4 || k5!=[v5,v15])").unwrap(),
            vec![
                AstNode::Comparison {
                    field: "k1".into(),
                    op: ComparisonOperator::Equal,
                    value: vec!["v1".into()],
                },
                AstNode::Op {
                    op: LogicalOperator::And,
                },
                AstNode::Group {
                    nodes: vec![
                        AstNode::Comparison {
                            field: "k2".into(),
                            op: ComparisonOperator::NotEqual,
                            value: vec!["v2".into()],
                        },
                        AstNode::Op {
                            op: LogicalOperator::Or,
                        },
                        AstNode::Comparison {
                            field: "k3".into(),
                            op: ComparisonOperator::Equal,
                            value: vec!["v3".into()],
                        }
                    ]
                },
                AstNode::Op {
                    op: LogicalOperator::And,
                },
                AstNode::Group {
                    nodes: vec![
                        AstNode::Comparison {
                            field: "k4".into(),
                            op: ComparisonOperator::Equal,
                            value: vec!["v4".into()],
                        },
                        AstNode::Op {
                            op: LogicalOperator::Or,
                        },
                        AstNode::Comparison {
                            field: "k5".into(),
                            op: ComparisonOperator::NotEqual,
                            value: vec!["v5".into(), "v15".into()],
                        }
                    ]
                },
            ],
        );
    }

    #[test]
    fn nested_group() {
        assert_eq!(
            parse_query("k1=v1 && (k2!=v2 OR k3=v3 || (k4=v4 AND k5!=[v5,v15]))").unwrap(),
            vec![
                AstNode::Comparison {
                    field: "k1".into(),
                    op: ComparisonOperator::Equal,
                    value: vec!["v1".into()],
                },
                AstNode::Op {
                    op: LogicalOperator::And,
                },
                AstNode::Group {
                    nodes: vec![
                        AstNode::Comparison {
                            field: "k2".into(),
                            op: ComparisonOperator::NotEqual,
                            value: vec!["v2".into()],
                        },
                        AstNode::Op {
                            op: LogicalOperator::Or,
                        },
                        AstNode::Comparison {
                            field: "k3".into(),
                            op: ComparisonOperator::Equal,
                            value: vec!["v3".into()],
                        },
                        AstNode::Op {
                            op: LogicalOperator::Or,
                        },
                        AstNode::Group {
                            nodes: vec![
                                AstNode::Comparison {
                                    field: "k4".into(),
                                    op: ComparisonOperator::Equal,
                                    value: vec!["v4".into()],
                                },
                                AstNode::Op {
                                    op: LogicalOperator::And,
                                },
                                AstNode::Comparison {
                                    field: "k5".into(),
                                    op: ComparisonOperator::NotEqual,
                                    value: vec!["v5".into(), "v15".into()],
                                }
                            ]
                        },
                    ]
                },
            ],
        );
    }

    #[test]
    fn id_patterns() {
        assert!(parse_query("key=id").is_ok());
        assert!(parse_query("key=id-dash").is_ok());
        assert!(parse_query("key=id_underscore").is_ok());
        assert!(parse_query("key=id/slash").is_ok());
        assert!(parse_query("key=id.period").is_ok());
    }

    #[test]
    fn like_glob_patterns() {
        assert_eq!(
            parse_query("key~value{foo,bar}").unwrap(),
            vec![AstNode::Comparison {
                field: "key".into(),
                op: ComparisonOperator::Like,
                value: vec!["value{foo,bar}".into()],
            }],
        );
        assert_eq!(
            parse_query("key !~ value[a-z]?").unwrap(),
            vec![AstNode::Comparison {
                field: "key".into(),
                op: ComparisonOperator::NotLike,
                value: vec!["value[a-z]?".into()],
            }],
        );
        assert_eq!(
            parse_query("key~value.*").unwrap(),
            vec![AstNode::Comparison {
                field: "key".into(),
                op: ComparisonOperator::Like,
                value: vec!["value.*".into()],
            }],
        );
        assert_eq!(
            parse_query("key!~value/**/*").unwrap(),
            vec![AstNode::Comparison {
                field: "key".into(),
                op: ComparisonOperator::NotLike,
                value: vec!["value/**/*".into()],
            }],
        );
    }
}
