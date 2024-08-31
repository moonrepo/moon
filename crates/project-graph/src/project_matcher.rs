use moon_project::Project;
use moon_query::{Condition, Criteria, Field, LogicalOperator};

pub fn matches_criteria(project: &Project, query: &Criteria) -> miette::Result<bool> {
    let match_all = matches!(query.op, LogicalOperator::And);
    let mut matched_any = false;

    for condition in &query.conditions {
        let matches = match condition {
            Condition::Field { field, .. } => {
                let result = match field {
                    Field::Language(langs) => condition.matches_enum(langs, &project.language),
                    Field::Project(ids) => {
                        if condition.matches(ids, &project.id)? {
                            Ok(true)
                        } else if let Some(alias) = &project.alias {
                            condition.matches(ids, alias)
                        } else {
                            Ok(false)
                        }
                    }
                    Field::ProjectAlias(aliases) => {
                        if let Some(alias) = &project.alias {
                            condition.matches(aliases, alias)
                        } else {
                            Ok(false)
                        }
                    }
                    Field::ProjectName(ids) => condition.matches(ids, &project.id),
                    Field::ProjectSource(sources) => {
                        condition.matches(sources, project.source.as_str())
                    }
                    Field::ProjectStack(types) => condition.matches_enum(types, &project.stack),
                    Field::ProjectType(types) => condition.matches_enum(types, &project.type_of),
                    Field::Tag(tags) => condition.matches_list(
                        tags,
                        &project
                            .config
                            .tags
                            .iter()
                            .map(|t| t.as_str())
                            .collect::<Vec<_>>(),
                    ),
                    Field::Task(ids) => Ok(project
                        .tasks
                        .values()
                        .any(|task| condition.matches(ids, &task.id).unwrap_or_default())),
                    Field::TaskPlatform(platforms) => Ok(project.tasks.values().any(|task| {
                        condition
                            .matches_enum(platforms, &task.platform)
                            .unwrap_or_default()
                    })),
                    Field::TaskType(types) => Ok(project.tasks.values().any(|task| {
                        condition
                            .matches_enum(types, &task.type_of)
                            .unwrap_or_default()
                    })),
                };

                result?
            }
            Condition::Criteria { criteria } => matches_criteria(project, criteria)?,
        };

        if matches {
            matched_any = true;

            if match_all {
                continue;
            } else {
                break;
            }
        } else if match_all {
            return Ok(false);
        }
    }

    // No matches using the OR condition
    if !matched_any {
        return Ok(false);
    }

    Ok(true)
}
