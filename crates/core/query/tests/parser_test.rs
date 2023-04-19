use moon_query::{parse, AstNode, ComparisonOperator, LogicalOperator};

mod mql_parse {
    use super::*;

    #[test]
    #[should_panic]
    fn errors_no_logic_op() {
        parse("k1=v1 k2=v2").unwrap();
    }

    #[test]
    #[should_panic]
    fn errors_invalid_eq() {
        parse("k1==v2").unwrap();
    }

    #[test]
    #[should_panic]
    fn errors_invalid_op() {
        parse("k1=v1 & k2=v2").unwrap();
    }

    #[test]
    #[should_panic]
    fn errors_double_logic_op() {
        parse("k1=v1 && && k2=v2").unwrap();
    }

    #[test]
    fn comp_eq() {
        assert_eq!(
            parse("key=value").unwrap(),
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
            parse("key=[v1, v2, v3]").unwrap(),
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
            parse("key!=value").unwrap(),
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
            parse("key!=[v1,v2,v3]").unwrap(),
            vec![AstNode::Comparison {
                field: "key".into(),
                op: ComparisonOperator::NotEqual,
                value: vec!["v1".into(), "v2".into(), "v3".into()],
            }],
        );
    }

    #[test]
    fn multi_and_comp() {
        assert_eq!(
            parse("k1=v1 && k2!=v2 AND k3=[1,2,3]").unwrap(),
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
            parse("k1=v1 || k2!=v2 OR k3=[1,2,3]").unwrap(),
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
            parse("key =  value").unwrap(),
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
            parse("k1=v1&&      k2!=v2").unwrap(),
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
            parse("k1=v1 && (k2 != v2 OR k3  =  v3)").unwrap(),
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
            parse("k1=v1 && (k2!=v2 OR k3=v3) AND (k4=v4 || k5!=[v5,v15])").unwrap(),
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
            parse("k1=v1 && (k2!=v2 OR k3=v3 || (k4=v4 AND k5!=[v5,v15]))").unwrap(),
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
}
