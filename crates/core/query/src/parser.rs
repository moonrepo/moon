use pest::{
    error::Error,
    iterators::{Pair, Pairs},
    Parser,
};
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "mql.pest"]
struct MqlParser;

#[derive(Clone, Debug, Default, PartialEq)]
pub enum LogicalOperator {
    #[default]
    And, // &&
    Or, // ||
}

#[derive(Debug, Default, PartialEq)]
pub enum ComparisonOperator {
    #[default]
    Equal, // =
    NotEqual, // !=
    Like,     // ~
    NotLike,  // !~
}

#[derive(Debug, PartialEq)]
pub enum AstNode {
    Comparison {
        field: String,
        op: ComparisonOperator,
        value: Vec<String>,
    },
    Op {
        op: LogicalOperator,
    },
    Group {
        nodes: Vec<AstNode>,
    },
}

fn parse_ast_node(pair: Pair<Rule>) -> Result<Option<AstNode>, Box<Error<Rule>>> {
    Ok(match pair.as_rule() {
        Rule::comparison => {
            let mut inner = pair.into_inner();
            let field = inner.next().expect("Missing field name.");
            let op = inner.next().expect("Missing comparison operator.");
            let value = inner.next().expect("Missing field value.");

            Some(AstNode::Comparison {
                field: match field.as_rule() {
                    Rule::key => field.as_str().to_string(),
                    _ => unreachable!(),
                },
                op: match op.as_rule() {
                    Rule::eq => ComparisonOperator::Equal,
                    Rule::neq => ComparisonOperator::NotEqual,
                    Rule::like => ComparisonOperator::Like,
                    Rule::nlike => ComparisonOperator::NotLike,
                    _ => unreachable!(),
                },
                value: match value.as_rule() {
                    Rule::value => vec![value.as_str().to_string()],
                    Rule::value_glob => vec![value.as_str().to_string()],
                    Rule::value_list => value
                        .into_inner()
                        .map(|pair| pair.as_str().to_string())
                        .collect(),
                    _ => unreachable!(),
                },
            })
        }
        Rule::expr_group => Some(AstNode::Group {
            nodes: parse_ast(pair.into_inner())?,
        }),
        Rule::and => Some(AstNode::Op {
            op: LogicalOperator::And,
        }),
        Rule::or => Some(AstNode::Op {
            op: LogicalOperator::Or,
        }),
        Rule::WHITESPACE | Rule::EOI => None,
        _ => unreachable!(),
    })
}

fn parse_ast(pairs: Pairs<Rule>) -> Result<Vec<AstNode>, Box<Error<Rule>>> {
    let mut ast = vec![];

    for pair in pairs {
        if let Some(node) = parse_ast_node(pair)? {
            ast.push(node);
        }
    }

    Ok(ast)
}

pub fn parse(input: &str) -> Result<Vec<AstNode>, Box<Error<Rule>>> {
    parse_ast(MqlParser::parse(Rule::query, input)?)
}
