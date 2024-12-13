use crate::tasks_builder_error::TasksBuilderError;
use moon_common::Id;
use moon_config::{DependencyConfig, TaskDependencyConfig};
use moon_task::{Target, TargetScope, Task, TaskOptions};
use std::mem;

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
    pub project_id: &'proj Id,
    pub project_dependencies: &'proj [DependencyConfig],
    pub task: &'proj mut Task,
}

impl TaskDepsBuilder<'_> {
    pub fn build(self) -> miette::Result<()> {
        let mut deps = vec![];

        for dep_config in mem::take(&mut self.task.deps) {
            let (project_ids, skip_if_missing) = match &dep_config.target.scope {
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
                    self.project_dependencies
                        .iter()
                        .map(|dep| &dep.id)
                        .collect::<Vec<_>>(),
                    dep_config.optional.unwrap_or(true),
                ),
                // ~:task
                TargetScope::OwnSelf => {
                    (vec![self.project_id], dep_config.optional.unwrap_or(false))
                }
                // id:task
                TargetScope::Project(project_id) => {
                    (vec![project_id], dep_config.optional.unwrap_or(false))
                }
                // #tag:task
                TargetScope::Tag(tag) => (
                    self.querent
                        .query_projects_by_tag(tag)?
                        .into_iter()
                        .filter(|id| *id != self.project_id)
                        .collect(),
                    dep_config.optional.unwrap_or(true),
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
                    .is_some_and(|id| id == self.project_id)
                    && dep_task_target.task_id == self.task.target.task_id
                {
                    continue;
                }

                self.check_and_push_dep(
                    dep_task_target,
                    dep_task_options,
                    &dep_config,
                    &mut deps,
                    skip_if_missing,
                )?;
            }
        }

        self.task.deps = deps;

        Ok(())
    }

    fn check_and_push_dep(
        &self,
        dep_task_target: &Target,
        dep_task_options: &TaskOptions,
        dep_config: &TaskDependencyConfig,
        deps_list: &mut Vec<TaskDependencyConfig>,
        _skip_if_missing: bool,
    ) -> miette::Result<()> {
        // Do not depend on tasks that can fail
        if dep_task_options.allow_failure {
            return Err(TasksBuilderError::AllowFailureDepRequirement {
                dep: dep_task_target.to_owned(),
                task: self.task.target.to_owned(),
            }
            .into());
        }

        // Do not depend on tasks that can't run in CI
        if !dep_task_options.run_in_ci.is_enabled() && self.task.options.run_in_ci.is_enabled() {
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

        if !deps_list.contains(&dep) {
            deps_list.push(dep);
        }

        Ok(())
    }
}
