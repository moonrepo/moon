use moon_config::{ProjectLanguage, ProjectType};
use pest::{error::Error, Parser};
use pest_derive::Parser;

// pub enum Operator {
//     Equal,    // =
//     NotEqual, // !=
//     Like,     // ~
//     NotLike,  // !~
// }

// #[derive(Debug, PartialEq)]
// pub struct ProjectQuery {
//     pub type_of: ProjectType,
// }

#[derive(Parser)]
#[grammar = "mql.pest"]
struct MqlParser;

pub enum Condition {
    All,
    Any,
}

pub struct ConditionGroup {
    pub condition: Condition,
    pub criteria: Vec<Condition>,
}

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

pub enum FieldComparison {
    Language(ComparisonOperator, ProjectLanguage),
    Project(ComparisonOperator, String),
    ProjectAlias(ComparisonOperator, String),
    ProjectSource(ComparisonOperator, String),
    ProjectType(ComparisonOperator, ProjectType),
}

pub fn parse(input: &str) -> Result<Vec<AstNode>, Error<Rule>> {
    let mut ast = vec![];
    let pairs = MqlParser::parse(Rule::query, input)?;

    dbg!(&pairs);

    for pair in pairs {
        match pair.as_rule() {
            Rule::comparison => {
                let mut inner = pair.into_inner();
                let field = inner.next().expect("Missing field name.");
                let op = inner.next().expect("Missing comparison operator.");
                let value = inner.next().expect("Missing field value.");

                dbg!("INNER", &inner);

                ast.push(AstNode::Comparison {
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
                });
            }
            Rule::and => {
                ast.push(AstNode::LogicalOp {
                    op: LogicalOperator::And,
                });
            }
            Rule::or => {
                ast.push(AstNode::LogicalOp {
                    op: LogicalOperator::Or,
                });
            }
            Rule::WHITESPACE | Rule::EOI => {}
            _ => unreachable!(),
        }
    }

    Ok(ast)
}

#[derive(Debug, PartialEq)]
pub enum AstNode {
    Comparison {
        field: String,
        op: ComparisonOperator,
        value: Vec<String>,
    },
    LogicalOp {
        op: LogicalOperator,
    },
}
