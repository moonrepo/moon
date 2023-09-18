use crate::parser::{parse_query, AstNode, ComparisonOperator, LogicalOperator};
use crate::query_error::QueryError;
use moon_config::{LanguageType, PlatformType, ProjectType, TaskType};
use starbase_utils::glob::GlobSet;
use std::cmp::PartialEq;
use std::str::FromStr;

#[derive(Debug, PartialEq)]
pub enum Field {
    Language(Vec<LanguageType>),
    Project(Vec<String>),
    ProjectAlias(Vec<String>),
    ProjectName(Vec<String>),
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
    pub fn matches(&self, haystack: &[String], needle: &String) -> miette::Result<bool> {
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

    pub fn matches_list(&self, haystack: &[String], needles: &[String]) -> miette::Result<bool> {
        for needle in needles {
            if self.matches(haystack, needle)? {
                return Ok(true);
            }
        }

        Ok(false)
    }

    pub fn matches_enum<T: PartialEq>(&self, haystack: &[T], needle: &T) -> miette::Result<bool> {
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
    pub input: Option<String>,
}

impl AsRef<Criteria> for Criteria {
    fn as_ref(&self) -> &Criteria {
        self
    }
}

fn build_criteria_enum<T: FromStr>(
    field: &str,
    op: &ComparisonOperator,
    values: Vec<String>,
) -> miette::Result<Vec<T>> {
    if matches!(op, ComparisonOperator::Like | ComparisonOperator::NotLike) {
        return Err(QueryError::UnsupportedLikeOperator(field.to_owned()).into());
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

fn build_criteria(ast: Vec<AstNode>) -> miette::Result<Criteria> {
    let mut op = None;
    let mut conditions = vec![];

    for node in ast {
        match node {
            AstNode::Comparison { field, op, value } => {
                let field = match field.as_str() {
                    "language" => {
                        Field::Language(build_criteria_enum::<LanguageType>(&field, &op, value)?)
                    }
                    "project" => Field::Project(value),
                    "projectAlias" => Field::ProjectAlias(value),
                    "projectName" => Field::ProjectName(value),
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
                        return Err(QueryError::UnknownField(field).into());
                    }
                };

                conditions.push(Condition::Field { field, op });
            }
            AstNode::Op { op: next_op } => {
                if let Some(current_op) = &op {
                    if &next_op != current_op {
                        return Err(QueryError::LogicalOperatorMismatch.into());
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
        input: None,
    })
}

pub fn build_query<I: AsRef<str>>(input: I) -> miette::Result<Criteria> {
    let input = input.as_ref();

    if input.is_empty() {
        return Err(QueryError::EmptyInput.into());
    }

    let mut criteria =
        build_criteria(parse_query(input).map_err(|e| QueryError::ParseFailure(e.to_string()))?)?;

    criteria.input = Some(input.to_owned());

    Ok(criteria)
}
