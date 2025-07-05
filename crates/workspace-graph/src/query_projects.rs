use crate::WorkspaceGraph;
use moon_common::{Id, color};
use moon_project_graph::Project;
use moon_query::*;
use std::{fmt::Debug, sync::Arc};
use tracing::{debug, instrument};

impl WorkspaceGraph {
    /// Return all expanded projects that match the query criteria.
    #[instrument(skip(self))]
    pub fn query_projects<'input, Q: AsRef<Criteria<'input>> + Debug>(
        &self,
        query: Q,
    ) -> miette::Result<Vec<Arc<Project>>> {
        let mut projects = vec![];

        for id in self.internal_query_projects(query)?.iter() {
            projects.push(self.get_project(id)?);
        }

        Ok(projects)
    }

    fn internal_query_projects<'input, Q: AsRef<Criteria<'input>>>(
        &self,
        query: Q,
    ) -> miette::Result<Arc<Vec<Id>>> {
        let query = query.as_ref();
        let query_input = query
            .input
            .as_ref()
            .expect("Querying the project graph requires a query input string.");
        let cache_key = query_input.to_string();

        if let Some(cache) = self.project_query_cache.read(&cache_key, |_, v| v.clone()) {
            return Ok(cache);
        }

        debug!("Querying projects with {}", color::shell(query_input));

        let mut project_ids = vec![];

        // Don't use `get_all` as it recursively calls `query`,
        // which runs into a deadlock! This should be faster also...
        for project in self.projects.get_all_unexpanded() {
            if self.does_project_match_criteria(project, query)? {
                project_ids.push(project.id.clone());
            }
        }

        // Sort so that the order is deterministic
        project_ids.sort();

        debug!(
            project_ids = ?project_ids
                .iter()
                .map(|id| id.as_str())
                .collect::<Vec<_>>(),
            "Found {} matches",
            project_ids.len(),
        );

        let ids = Arc::new(project_ids);
        let _ = self.project_query_cache.insert(cache_key, Arc::clone(&ids));

        Ok(ids)
    }

    fn does_project_match_criteria(
        &self,
        project: &Project,
        query: &Criteria,
    ) -> miette::Result<bool> {
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
                        Field::ProjectLayer(types) | Field::ProjectType(types) => {
                            condition.matches_enum(types, &project.layer)
                        }
                        Field::ProjectName(ids) => condition.matches(ids, &project.id),
                        Field::ProjectSource(sources) => {
                            condition.matches(sources, project.source.as_str())
                        }
                        Field::ProjectStack(types) => condition.matches_enum(types, &project.stack),
                        Field::Tag(tags) => condition.matches_list(
                            tags,
                            &project
                                .config
                                .tags
                                .iter()
                                .map(|t| t.as_str())
                                .collect::<Vec<_>>(),
                        ),
                        Field::Task(ids) => Ok(project.task_targets.iter().any(|target| {
                            condition.matches(ids, &target.task_id).unwrap_or_default()
                        })),
                        Field::TaskPlatform(ids) | Field::TaskToolchain(ids) => Ok(self
                            .tasks
                            .get_all_for_project(&project.id, false)?
                            .iter()
                            .any(|task| {
                                let toolchains = task
                                    .toolchains
                                    .iter()
                                    .map(|t| t.as_str())
                                    .collect::<Vec<_>>();

                                condition.matches_list(ids, &toolchains).unwrap_or_default()
                            })),
                        Field::TaskType(types) => Ok(self
                            .tasks
                            .get_all_for_project(&project.id, false)?
                            .iter()
                            .any(|task| {
                                condition
                                    .matches_enum(types, &task.type_of)
                                    .unwrap_or_default()
                            })),
                    };

                    result?
                }
                Condition::Criteria { criteria } => {
                    self.does_project_match_criteria(project, criteria)?
                }
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
}
