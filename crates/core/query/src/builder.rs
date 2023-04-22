use crate::errors::QueryError;
use crate::parser::{parse, AstNode, ComparisonOperator, LogicalOperator};
use moon_config::{PlatformType, ProjectLanguage, ProjectType, TaskType};
use starbase_utils::glob::{GlobError, GlobSet};
use std::cmp::PartialEq;
use std::str::FromStr;

#[derive(Debug, PartialEq)]
pub enum Field {
    Language(Vec<ProjectLanguage>),
    Project(Vec<String>),
    ProjectAlias(Vec<String>),
    ProjectSource(Vec<String>),
    ProjectType(Vec<ProjectType>),
    Tag(Vec<String>),
    Task(Vec<String>),
    TaskPlatform(Vec<PlatformType>),
    TaskType(Vec<TaskType>),
}

#[derive(Debug, PartialEq)]
pub enum Condition {
    Field {
        field: Field,
        op: ComparisonOperator,
    },
    Criteria {
        criteria: Criteria,
    },
}

impl Condition {
    pub fn matches(&self, haystack: &[String], needle: &String) -> Result<bool, GlobError> {
        Ok(match self {
            Condition::Field { op, .. } => match op {
                ComparisonOperator::Equal => haystack.contains(needle),
                ComparisonOperator::NotEqual => !haystack.contains(needle),
                ComparisonOperator::Like => GlobSet::new(haystack)?.is_match(needle),
                ComparisonOperator::NotLike => !GlobSet::new(haystack)?.is_match(needle),
            },
            Condition::Criteria { .. } => false,
        })
    }

    pub fn matches_list(&self, haystack: &[String], needles: &[String]) -> Result<bool, GlobError> {
        for needle in needles {
            if self.matches(haystack, needle)? {
                return Ok(true);
            }
        }

        Ok(false)
    }

    pub fn matches_enum<T: PartialEq>(
        &self,
        haystack: &[T],
        needle: &T,
    ) -> Result<bool, GlobError> {
        Ok(match self {
            Condition::Field { op, .. } => match op {
                ComparisonOperator::Equal => haystack.contains(needle),
                ComparisonOperator::NotEqual => !haystack.contains(needle),
                // Like and NotLike are not supported for enums
                _ => false,
            },
            Condition::Criteria { .. } => false,
        })
    }
}

#[derive(Debug, Default, PartialEq)]
pub struct Criteria {
    pub op: LogicalOperator,
    pub conditions: Vec<Condition>,
}

fn build_criteria_enum<T: FromStr>(
    field: &str,
    op: &ComparisonOperator,
    values: Vec<String>,
) -> Result<Vec<T>, QueryError> {
    if matches!(op, ComparisonOperator::Like | ComparisonOperator::NotLike) {
        return Err(QueryError::UnsupportedLikeOperator(field.to_owned()));
    }

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

fn build_criteria(ast: Vec<AstNode>) -> Result<Criteria, QueryError> {
    let mut op = None;
    let mut conditions = vec![];

    for node in ast {
        match node {
            AstNode::Comparison { field, op, value } => {
                let field = match field.as_str() {
                    "language" => {
                        Field::Language(build_criteria_enum::<ProjectLanguage>(&field, &op, value)?)
                    }
                    "project" => Field::Project(value),
                    "projectAlias" => Field::ProjectAlias(value),
                    "projectSource" => Field::ProjectSource(value),
                    "projectType" => {
                        Field::ProjectType(build_criteria_enum::<ProjectType>(&field, &op, value)?)
                    }
                    "tag" => Field::Tag(value),
                    "task" => Field::Task(value),
                    "taskPlatform" => Field::TaskPlatform(build_criteria_enum::<PlatformType>(
                        &field, &op, value,
                    )?),
                    "taskType" => {
                        Field::TaskType(build_criteria_enum::<TaskType>(&field, &op, value)?)
                    }
                    _ => {
                        return Err(QueryError::UnknownField(field));
                    }
                };

                conditions.push(Condition::Field { field, op });
            }
            AstNode::Op { op: next_op } => {
                if let Some(current_op) = &op {
                    if &next_op != current_op {
                        return Err(QueryError::LogicalOperatorMismatch);
                    }
                } else {
                    op = Some(next_op);
                }
            }
            AstNode::Group { nodes } => {
                conditions.push(Condition::Criteria {
                    criteria: build_criteria(nodes)?,
                });
            }
        }
    }

    Ok(Criteria {
        op: op.unwrap_or_default(),
        conditions,
    })
}

pub fn build(input: &str) -> Result<Criteria, QueryError> {
    if input.is_empty() {
        return Err(QueryError::EmptyInput);
    }

    build_criteria(parse(input).map_err(|e| QueryError::ParseFailure(e.to_string()))?)
}
