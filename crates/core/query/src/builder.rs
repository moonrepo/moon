use crate::errors::QueryError;
use crate::parser::{parse, AstNode, ComparisonOperator, LogicalOperator};
use moon_config::{PlatformType, ProjectLanguage, ProjectType, TaskType};
use std::str::FromStr;

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

pub struct QueryField {
    pub field: Field,
    pub op: ComparisonOperator,
}

pub struct QueryCriteria {
    pub op: Option<LogicalOperator>,
    pub fields: Vec<QueryField>,
    pub criteria: Vec<QueryCriteria>,
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
    build_criteria(parse(input).map_err(|e| QueryError::ParseFailure(e.to_string()))?)
}
