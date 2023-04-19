use std::str::FromStr;

use crate::errors::QueryError;
use moon_config::{ProjectLanguage, ProjectType};
use pest::{
    error::Error,
    iterators::{Pair, Pairs},
    Parser,
};
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "mql.pest"]
struct MqlParser;

#[derive(Debug, PartialEq)]
pub enum LogicalOperator {
    And, // &&
    Or,  // ||
}

#[derive(Debug, PartialEq)]
pub enum ComparisonOperator {
    Equal,    // =
    NotEqual, // !=
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

fn build_ast_node(pair: Pair<Rule>) -> Result<Option<AstNode>, Error<Rule>> {
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
                    _ => unreachable!(),
                },
                value: match value.as_rule() {
                    Rule::value => vec![value.as_str().to_string()],
                    Rule::value_list => value
                        .into_inner()
                        .map(|pair| pair.as_str().to_string())
                        .collect(),
                    _ => unreachable!(),
                },
            })
        }
        Rule::expr_group => Some(AstNode::Group {
            nodes: build_ast(pair.into_inner())?,
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

fn build_ast(pairs: Pairs<Rule>) -> Result<Vec<AstNode>, Error<Rule>> {
    let mut ast = vec![];

    for pair in pairs {
        if let Some(node) = build_ast_node(pair)? {
            ast.push(node);
        }
    }

    Ok(ast)
}

pub fn parse(input: &str) -> Result<Vec<AstNode>, Error<Rule>> {
    Ok(build_ast(MqlParser::parse(Rule::query, input)?)?)
}

pub enum Field {
    Language(Vec<ProjectLanguage>),
    Project(Vec<String>),
    ProjectAlias(Vec<String>),
    ProjectSource(Vec<String>),
    ProjectType(Vec<ProjectType>),
}

pub struct QueryField {
    pub field: Field,
    pub op: ComparisonOperator,
}

pub struct QueryCriteria {
    pub op: Option<LogicalOperator>,
    pub fields: Vec<QueryField>,
    pub criteria: Vec<QueryCriteria>,
}

fn build_criteria_values<T: FromStr>(
    field: &str,
    values: Vec<String>,
) -> Result<Vec<T>, QueryError> {
    let mut result = vec![];

    for value in values {
        result.push(
            value
                .parse()
                .map_err(|_| QueryError::UnknownFieldValue(field.to_owned(), value))?,
        );
    }

    Ok(result)
}

fn build_criteria(ast: Vec<AstNode>) -> Result<QueryCriteria, QueryError> {
    let mut criteria = QueryCriteria {
        op: None,
        fields: vec![],
        criteria: vec![],
    };

    for node in ast {
        match node {
            AstNode::Comparison { field, op, value } => {
                let field = match field.as_str() {
                    "language" => {
                        Field::Language(build_criteria_values::<ProjectLanguage>(&field, value)?)
                    }
                    "project" => Field::Project(value),
                    "projectAlias" => Field::ProjectAlias(value),
                    "projectSource" => Field::ProjectSource(value),
                    "projectType" => {
                        Field::ProjectType(build_criteria_values::<ProjectType>(&field, value)?)
                    }
                    _ => {
                        return Err(QueryError::UnknownField(field));
                    }
                };

                criteria.fields.push(QueryField { field, op });
            }
            AstNode::Op { op } => {
                if let Some(current_op) = &criteria.op {
                    if &op != current_op {
                        return Err(QueryError::LogicalOperatorMismatch);
                    }
                } else {
                    criteria.op = Some(op);
                }
            }
            AstNode::Group { nodes } => {
                criteria.criteria.push(build_criteria(nodes)?);
            }
        }
    }

    Ok(criteria)
}

pub fn build(input: &str) -> Result<QueryCriteria, QueryError> {
    let ast = parse(input).map_err(|e| QueryError::ParseFailure(e.to_string()))?;

    Ok(build_criteria(ast)?)
}
