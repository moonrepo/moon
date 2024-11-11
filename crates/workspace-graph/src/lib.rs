use moon_common::{color, Id};
use moon_project_graph::{Project, ProjectGraph};
use moon_query::*;
use moon_task_graph::{Target, Task, TaskGraph};
use scc::HashMap;
use std::{fmt::Debug, path::Path, sync::Arc};
use tracing::{debug, instrument};

pub use moon_graph_utils::*;
pub use moon_project_graph as projects;
pub use moon_task_graph as tasks;

#[derive(Clone, Default)]
pub struct WorkspaceGraph {
    pub projects: Arc<ProjectGraph>,
    pub tasks: Arc<TaskGraph>,

    /// Cache of query results, mapped by query input to project IDs.
    query_cache: HashMap<String, Arc<Vec<Id>>>,
}

impl WorkspaceGraph {
    pub fn new(projects: Arc<ProjectGraph>, tasks: Arc<TaskGraph>) -> Self {
        Self {
            projects,
            tasks,
            query_cache: HashMap::default(),
        }
    }

    pub fn get_project(&self, id_or_alias: impl AsRef<str>) -> miette::Result<Arc<Project>> {
        self.projects.get(id_or_alias.as_ref())
    }

    pub fn get_project_from_path(
        &self,
        starting_file: Option<&Path>,
    ) -> miette::Result<Arc<Project>> {
        self.projects.get_from_path(starting_file)
    }

    pub fn get_project_with_tasks(&self, id_or_alias: impl AsRef<str>) -> miette::Result<Project> {
        let base_project = self.get_project(id_or_alias)?;
        let mut project = base_project.as_ref().to_owned();

        for base_task in self.get_tasks_from_project(&project.id)? {
            project
                .tasks
                .insert(base_task.id.clone(), base_task.as_ref().to_owned());
        }

        Ok(project)
    }

    pub fn get_all_projects(&self) -> miette::Result<Vec<Arc<Project>>> {
        self.projects.get_all()
    }

    pub fn get_task(&self, target: &Target) -> miette::Result<Arc<Task>> {
        self.tasks.get(target)
    }

    pub fn get_task_from_project(
        &self,
        project_id: impl AsRef<str>,
        task_id: impl AsRef<str>,
    ) -> miette::Result<Arc<Task>> {
        let target = Target::new(project_id, task_id)?;

        self.tasks.get(&target)
    }

    pub fn get_tasks_from_project(
        &self,
        project_id: impl AsRef<str>,
    ) -> miette::Result<Vec<Arc<Task>>> {
        let project = self.get_project(project_id)?;
        let mut all = vec![];

        for target in &project.task_targets {
            let task = self.get_task(target)?;

            if !task.is_internal() {
                all.push(task);
            }
        }

        Ok(all)
    }

    pub fn get_all_tasks(&self) -> miette::Result<Vec<Arc<Task>>> {
        Ok(self
            .tasks
            .get_all()?
            .into_iter()
            .filter(|task| !task.is_internal())
            .collect())
    }
}

impl WorkspaceGraph {
    /// Return all expanded projects that match the query criteria.
    #[instrument(name = "query_projects", skip(self))]
    pub fn query_projects<'input, Q: AsRef<Criteria<'input>> + Debug>(
        &self,
        query: Q,
    ) -> miette::Result<Vec<Arc<Project>>> {
        let mut projects = vec![];

        for id in self.internal_query(query)?.iter() {
            projects.push(self.get_project(id)?);
        }

        Ok(projects)
    }

    fn internal_query<'input, Q: AsRef<Criteria<'input>>>(
        &self,
        query: Q,
    ) -> miette::Result<Arc<Vec<Id>>> {
        let query = query.as_ref();
        let query_input = query
            .input
            .as_ref()
            .expect("Querying the project graph requires a query input string.");
        let cache_key = query_input.to_string();

        if let Some(cache) = self.query_cache.read(&cache_key, |_, v| v.clone()) {
            return Ok(cache);
        }

        debug!("Querying projects with {}", color::shell(query_input));

        let mut project_ids = vec![];

        // Don't use `get_all` as it recursively calls `query`,
        // which runs into a deadlock! This should be faster also...
        for project in self.projects.get_all_unexpanded() {
            if self.matches_criteria(project, query)? {
                project_ids.push(project.id.clone());
            }
        }

        // Sort so that the order is deterministic
        project_ids.sort();

        debug!(
            projects = ?project_ids
                .iter()
                .map(|id| id.as_str())
                .collect::<Vec<_>>(),
            "Found {} matches",
            project_ids.len(),
        );

        let ids = Arc::new(project_ids);
        let _ = self.query_cache.insert(cache_key, Arc::clone(&ids));

        Ok(ids)
    }

    fn matches_criteria(&self, project: &Project, query: &Criteria) -> miette::Result<bool> {
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
                        Field::ProjectType(types) => {
                            condition.matches_enum(types, &project.type_of)
                        }
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
                        Field::TaskPlatform(platforms) => Ok(self
                            .tasks
                            .get_all_for_project(&project.id, false)?
                            .iter()
                            .any(|task| {
                                condition
                                    .matches_enum(platforms, &task.platform)
                                    .unwrap_or_default()
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
                Condition::Criteria { criteria } => self.matches_criteria(project, criteria)?,
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
