use crate::parser::{AstNode, ComparisonOperator, LogicalOperator, parse_query};
use crate::query_error::QueryError;
use moon_common::color;
use moon_config::{LanguageType, LayerType, StackType, TaskType};
use starbase_utils::glob::GlobSet;
use std::borrow::Cow;
use std::cmp::PartialEq;
use std::str::FromStr;
use tracing::{debug, instrument};

pub type FieldValue<'l> = Cow<'l, str>;
pub type FieldValues<'l> = Vec<FieldValue<'l>>;

#[derive(Debug, PartialEq)]
pub enum Field<'l> {
    Language(Vec<LanguageType>),
    Project(FieldValues<'l>),
    ProjectAlias(FieldValues<'l>),
    ProjectLayer(Vec<LayerType>),
    ProjectName(FieldValues<'l>),
    ProjectSource(FieldValues<'l>),
    ProjectStack(Vec<StackType>),
    ProjectType(Vec<LayerType>),
    Tag(FieldValues<'l>),
    Task(FieldValues<'l>),
    TaskPlatform(FieldValues<'l>),
    TaskToolchain(FieldValues<'l>),
    TaskType(Vec<TaskType>),
}

#[derive(Debug, PartialEq)]
pub enum Condition<'l> {
    Field {
        field: Field<'l>,
        op: ComparisonOperator,
    },
    Criteria {
        criteria: Criteria<'l>,
    },
}

impl Condition<'_> {
    pub fn matches(&self, haystack: &FieldValues, needle: &str) -> miette::Result<bool> {
        Ok(match self {
            Condition::Field { op, .. } => match op {
                ComparisonOperator::Equal => haystack.contains(&Cow::Borrowed(needle)),
                ComparisonOperator::NotEqual => !haystack.contains(&Cow::Borrowed(needle)),
                ComparisonOperator::Like => GlobSet::new(haystack)?.is_match(needle),
                ComparisonOperator::NotLike => !GlobSet::new(haystack)?.is_match(needle),
            },
            Condition::Criteria { .. } => false,
        })
    }

    pub fn matches_list(&self, haystack: &FieldValues, needles: &[&str]) -> miette::Result<bool> {
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
pub struct Criteria<'l> {
    pub op: LogicalOperator,
    pub conditions: Vec<Condition<'l>>,
    pub input: Option<Cow<'l, str>>,
}

impl<'l> AsRef<Criteria<'l>> for Criteria<'l> {
    fn as_ref(&self) -> &Criteria<'l> {
        self
    }
}

fn build_criteria_enum<T: FromStr>(
    field: &str,
    op: &ComparisonOperator,
    values: FieldValues<'_>,
) -> miette::Result<Vec<T>> {
    if matches!(op, ComparisonOperator::Like | ComparisonOperator::NotLike) {
        return Err(QueryError::UnsupportedLikeOperator(field.to_owned()).into());
    }

    let mut result = vec![];

    for value in values {
        result.push(
            value
                .parse()
                .map_err(|_| QueryError::UnknownFieldValue(field.to_owned(), value.to_string()))?,
        );
    }

    Ok(result)
}

fn build_criteria(ast: Vec<AstNode<'_>>) -> miette::Result<Criteria<'_>> {
    let mut op = None;
    let mut conditions = vec![];

    for node in ast {
        match node {
            AstNode::Comparison { field, op, value } => {
                let field = match field.as_ref() {
                    "language" => {
                        Field::Language(build_criteria_enum::<LanguageType>(&field, &op, value)?)
                    }
                    "project" => Field::Project(value),
                    "projectAlias" => Field::ProjectAlias(value),
                    "projectLayer" => {
                        Field::ProjectLayer(build_criteria_enum::<LayerType>(&field, &op, value)?)
                    }
                    "projectName" => Field::ProjectName(value),
                    "projectSource" => Field::ProjectSource(value),
                    "projectStack" => {
                        Field::ProjectStack(build_criteria_enum::<StackType>(&field, &op, value)?)
                    }
                    "projectType" => {
                        Field::ProjectType(build_criteria_enum::<LayerType>(&field, &op, value)?)
                    }
                    "tag" => Field::Tag(value),
                    "task" => Field::Task(value),
                    "taskPlatform" => {
                        debug!(
                            "The {} query field is deprecated, use {} instead",
                            color::property("taskPlatform"),
                            color::property("taskToolchain"),
                        );

                        Field::TaskPlatform(value)
                    }
                    "taskToolchain" => Field::TaskToolchain(value),
                    "taskType" => {
                        Field::TaskType(build_criteria_enum::<TaskType>(&field, &op, value)?)
                    }
                    _ => {
                        return Err(QueryError::UnknownField(field.to_string()).into());
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

#[instrument]
pub fn build_query(input: &str) -> miette::Result<Criteria<'_>> {
    if input.is_empty() {
        return Err(QueryError::EmptyInput.into());
    }

    let mut criteria =
        build_criteria(parse_query(input).map_err(|e| QueryError::ParseFailure(e.to_string()))?)?;

    criteria.input = Some(Cow::Borrowed(input));

    Ok(criteria)
}
