use crate::WorkspaceGraph;
use moon_common::color;
use moon_query::*;
use moon_task_graph::{Target, Task};
use std::{fmt::Debug, sync::Arc};
use tracing::{debug, instrument};

impl WorkspaceGraph {
    /// Return all expanded tasks that match the query criteria.
    #[instrument(name = "query_tasks", skip(self))]
    pub fn query_tasks<'input, Q: AsRef<Criteria<'input>> + Debug>(
        &self,
        query: Q,
    ) -> miette::Result<Vec<Arc<Task>>> {
        let mut tasks = vec![];

        for target in self.internal_query_tasks(query)?.iter() {
            tasks.push(self.get_task(target)?);
        }

        Ok(tasks)
    }

    fn internal_query_tasks<'input, Q: AsRef<Criteria<'input>>>(
        &self,
        query: Q,
    ) -> miette::Result<Arc<Vec<Target>>> {
        let query = query.as_ref();
        let query_input = query
            .input
            .as_ref()
            .expect("Querying the task graph requires a query input string.");
        let cache_key = query_input.to_string();

        if let Some(cache) = self.task_query_cache.read(&cache_key, |_, v| v.clone()) {
            return Ok(cache);
        }

        debug!("Querying tasks with {}", color::shell(query_input));

        let mut targets = vec![];

        // Don't use `get_all` as it recursively calls `query`,
        // which runs into a deadlock! This should be faster also...
        for task in self.tasks.get_all_unexpanded() {
            if does_task_match_criteria(task, query)? {
                targets.push(task.target.clone());
            }
        }

        // Sort so that the order is deterministic
        targets.sort();

        debug!(
            task_targets = ?targets
                .iter()
                .map(|target| target.as_str())
                .collect::<Vec<_>>(),
            "Found {} matches",
            targets.len(),
        );

        let targets = Arc::new(targets);
        let _ = self
            .task_query_cache
            .insert(cache_key, Arc::clone(&targets));

        Ok(targets)
    }
}

fn does_task_match_criteria(task: &Task, query: &Criteria) -> miette::Result<bool> {
    let match_all = matches!(query.op, LogicalOperator::And);
    let mut matched_any = false;

    for condition in &query.conditions {
        let matches = match condition {
            Condition::Field { field, .. } => {
                let result = match field {
                    Field::Project(ids) => {
                        if let Some(project_id) = task.target.get_project_id() {
                            condition.matches(ids, project_id)
                        } else {
                            Ok(false)
                        }
                    }
                    Field::Task(ids) => condition.matches(ids, &task.id),
                    Field::TaskPlatform(ids) | Field::TaskToolchain(ids) => {
                        let toolchains = task
                            .toolchains
                            .iter()
                            .map(|t| t.as_str())
                            .collect::<Vec<_>>();

                        condition.matches_list(ids, &toolchains)
                    }
                    Field::TaskType(types) => condition.matches_enum(types, &task.type_of),
                    _ => Ok(false),
                };

                result?
            }
            Condition::Criteria { criteria } => does_task_match_criteria(task, criteria)?,
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
