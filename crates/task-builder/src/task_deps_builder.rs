use crate::tasks_builder_error::TasksBuilderError;
use moon_common::Id;
use moon_config::{
    DependencyScope, DependencySource, ProjectDependencyConfig, TaskDependencyConfig,
};
use moon_project::Project;
use moon_task::{Target, TargetScope, Task, TaskOptionRunInCI, TaskOptions};
use std::mem;
use tracing::trace;

pub trait TasksQuerent {
    fn query_projects_by_tag(&self, tag: &str) -> miette::Result<Vec<&Id>>;
    fn query_tasks(
        &self,
        project_ids: Vec<&Id>,
        task_id: &Id,
    ) -> miette::Result<Vec<(&Target, &TaskOptions)>>;
}

pub struct TaskDepsBuilder<'proj> {
    pub querent: Box<dyn TasksQuerent + 'proj>,
    pub project: Option<&'proj mut Project>,
    pub root_project_id: Option<&'proj Id>,
    pub task: &'proj mut Task,
}

impl TaskDepsBuilder<'_> {
    pub fn build(mut self) -> miette::Result<()> {
        let mut deps = vec![];
        let project = self.project.take().unwrap();

        for dep_config in mem::take(&mut self.task.deps) {
            let (project_ids, skip_if_missing, link_implicit_project_deps) =
                match &dep_config.target.scope {
                    // :task
                    TargetScope::All => {
                        return Err(TasksBuilderError::UnsupportedTargetScopeInDeps {
                            dep: dep_config.target.to_owned(),
                            task: self.task.target.to_owned(),
                        }
                        .into());
                    }
                    // ^:task
                    TargetScope::Deps => (
                        project
                            .dependencies
                            .iter()
                            .map(|dep| &dep.id)
                            .collect::<Vec<_>>(),
                        dep_config.optional.unwrap_or(true),
                        false,
                    ),
                    // ~:task
                    TargetScope::OwnSelf => (
                        vec![&project.id],
                        dep_config.optional.unwrap_or(false),
                        false,
                    ),
                    // id:task
                    TargetScope::Project(project_id) => {
                        (vec![project_id], dep_config.optional.unwrap_or(false), true)
                    }
                    // #tag:task
                    TargetScope::Tag(tag) => (
                        self.querent
                            .query_projects_by_tag(tag)?
                            .into_iter()
                            .filter(|id| *id != &project.id)
                            .collect(),
                        dep_config.optional.unwrap_or(true),
                        true,
                    ),
                };

            let results = self
                .querent
                .query_tasks(project_ids, &dep_config.target.task_id)?;

            if results.is_empty() && !skip_if_missing {
                return Err(match &dep_config.target.scope {
                    TargetScope::Deps => TasksBuilderError::UnknownDepTargetParentScope {
                        dep: dep_config.target.to_owned(),
                        task: self.task.target.to_owned(),
                    }
                    .into(),
                    TargetScope::Tag(_) => TasksBuilderError::UnknownDepTargetTagScope {
                        dep: dep_config.target.to_owned(),
                        task: self.task.target.to_owned(),
                    }
                    .into(),
                    _ => TasksBuilderError::UnknownDepTarget {
                        dep: dep_config.target.to_owned(),
                        task: self.task.target.to_owned(),
                    }
                    .into(),
                });
            }

            for (dep_task_target, dep_task_options) in results {
                // Avoid circular references
                if dep_task_target
                    .get_project_id()
                    .is_ok_and(|id| id == &project.id)
                    && dep_task_target.task_id == self.task.target.task_id
                {
                    continue;
                }

                let task_dep =
                    self.check_and_create_dep(dep_task_target, dep_task_options, &dep_config)?;

                if link_implicit_project_deps
                    && let Some(project_dep) = create_project_dep_from_task_dep(
                        &task_dep,
                        &project.id,
                        self.root_project_id,
                        |dep_project_id| {
                            project
                                .alias
                                .as_ref()
                                .is_some_and(|alias| alias.as_str() == dep_project_id.as_str())
                                || project
                                    .dependencies
                                    .iter()
                                    .any(|pd| &pd.id == dep_project_id)
                        },
                    )
                {
                    project.dependencies.push(project_dep);
                }

                if !deps.contains(&task_dep) {
                    deps.push(task_dep);
                }
            }
        }

        self.task.deps = deps;

        Ok(())
    }

    fn check_and_create_dep(
        &self,
        dep_task_target: &Target,
        dep_task_options: &TaskOptions,
        dep_config: &TaskDependencyConfig,
    ) -> miette::Result<TaskDependencyConfig> {
        // Do not depend on tasks that can fail
        if dep_task_options.allow_failure {
            return Err(TasksBuilderError::AllowFailureDepRequirement {
                dep: dep_task_target.to_owned(),
                task: self.task.target.to_owned(),
            }
            .into());
        }

        // Do not depend on tasks that can't run in CI
        if !dep_task_options.run_in_ci.is_enabled()
            && self.task.options.run_in_ci.is_enabled()
            && dep_task_options.run_in_ci != TaskOptionRunInCI::Skip
            && self.task.options.run_in_ci != TaskOptionRunInCI::Skip
        {
            return Err(TasksBuilderError::RunInCiDepRequirement {
                dep: dep_task_target.to_owned(),
                task: self.task.target.to_owned(),
            }
            .into());
        }

        // Enforce persistent constraints
        if dep_task_options.persistent && !self.task.options.persistent {
            return Err(TasksBuilderError::PersistentDepRequirement {
                dep: dep_task_target.to_owned(),
                task: self.task.target.to_owned(),
            }
            .into());
        }

        // Add the dep if it has not already been
        let dep = TaskDependencyConfig {
            target: dep_task_target.to_owned(),
            // optional: Some(skip_if_missing),
            ..dep_config.clone()
        };

        Ok(dep)
    }
}

pub fn create_project_dep_from_task_dep(
    task_dep: &TaskDependencyConfig,
    project_id: &Id,
    root_project_id: Option<&Id>,
    already_exists: impl FnOnce(&Id) -> bool,
) -> Option<ProjectDependencyConfig> {
    let TargetScope::Project(dep_project_id) = &task_dep.target.scope else {
        return None;
    };

    // Already a dependency, or references self
    if project_id == dep_project_id || already_exists(dep_project_id) {
        return None;
    }

    trace!(
        project_id = project_id.as_str(),
        dep_id = dep_project_id.as_str(),
        task_target = task_dep.target.as_str(),
        "Marking arbitrary project as an implicit dependency because of a task dependency"
    );

    Some(ProjectDependencyConfig {
        id: dep_project_id.to_owned(),
        scope: if root_project_id.is_some_and(|id| id == dep_project_id) {
            DependencyScope::Root
        } else {
            DependencyScope::Build
        },
        source: DependencySource::Implicit,
        via: Some(format!("task {}", task_dep.target)),
    })
}
