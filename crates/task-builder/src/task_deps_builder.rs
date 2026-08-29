use crate::tasks_builder_error::TasksBuilderError;
use moon_common::Id;
use moon_config::{
    DependencyScope, DependencySource, ProjectDependencyConfig, TaskDependencyCacheStrategy,
    TaskDependencyConfig, TaskDependencyType,
};
use moon_project::Project;
use moon_task::{
    Target, TargetDependencyScope as TargetDepScope, TargetProjectScope, TargetTaskScope, Task,
    TaskOptionRunInCI, TaskOptions,
};
use rustc_hash::FxHashMap;
use std::mem;
use tracing::debug;

fn normalize_type(type_of: TaskDependencyType) -> TaskDependencyType {
    if type_of.is_required_type() {
        TaskDependencyType::Required
    } else {
        type_of
    }
}

pub trait TasksQuerent {
    fn query_projects_by_tag(&self, tag: &str) -> miette::Result<Vec<&Id>>;
    fn query_tasks(
        &self,
        project_ids: Vec<&Id>,
        task_scope: (TargetTaskScope, &str),
    ) -> miette::Result<Vec<(&Target, &TaskOptions)>>;
    fn query_task_has_outputs(&self, target: &Target) -> bool;
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
        let mut dep_types = FxHashMap::<Target, TaskDependencyType>::default();
        let project = self.project.take().unwrap();

        for dep_config in mem::take(&mut self.task.deps) {
            let (project_ids, skip_if_missing, link_implicit_project_deps) = {
                let (scope, scope_value) = dep_config.target.get_project_scope();

                match scope {
                    // :task
                    TargetProjectScope::All => {
                        return Err(TasksBuilderError::UnsupportedTargetScopeInDeps {
                            dep: dep_config.target.to_owned(),
                            task: self.task.target.to_owned(),
                        }
                        .into());
                    }
                    // ^:task
                    TargetProjectScope::Deps => (
                        project
                            .dependencies
                            .iter()
                            .map(|dep| dep.id.clone())
                            .collect::<Vec<_>>(),
                        dep_config.optional.unwrap_or(true),
                        false,
                    ),
                    // ^build:task, ^development:task, etc
                    TargetProjectScope::DepsOf(scope) => {
                        let config_scope = match scope {
                            TargetDepScope::Build => DependencyScope::Build,
                            TargetDepScope::Development => DependencyScope::Development,
                            TargetDepScope::Peer => DependencyScope::Peer,
                            TargetDepScope::Production => DependencyScope::Production,
                        };

                        (
                            project
                                .dependencies
                                .iter()
                                .filter_map(|dep| {
                                    if dep.scope == config_scope {
                                        Some(dep.id.clone())
                                    } else {
                                        None
                                    }
                                })
                                .collect::<Vec<_>>(),
                            dep_config.optional.unwrap_or(true),
                            false,
                        )
                    }
                    // ~:task
                    TargetProjectScope::OwnSelf => (
                        vec![project.id.clone()],
                        dep_config.optional.unwrap_or(false),
                        false,
                    ),
                    // id:task
                    TargetProjectScope::Id => (
                        vec![Id::raw(scope_value)],
                        dep_config.optional.unwrap_or(false),
                        true,
                    ),
                    // #tag:task
                    TargetProjectScope::Tag => (
                        self.querent
                            .query_projects_by_tag(scope_value)?
                            .into_iter()
                            .filter(|id| *id != &project.id)
                            .cloned()
                            .collect(),
                        dep_config.optional.unwrap_or(true),
                        true,
                    ),
                }
            };

            let results = self.querent.query_tasks(
                project_ids.iter().collect(),
                dep_config.target.get_task_scope(),
            )?;

            if results.is_empty() && !skip_if_missing {
                return Err(match &dep_config.target.project {
                    TargetProjectScope::Deps | TargetProjectScope::DepsOf(_) => {
                        TasksBuilderError::UnknownDepTargetParentScope {
                            dep: dep_config.target.to_owned(),
                            task: self.task.target.to_owned(),
                        }
                        .into()
                    }
                    TargetProjectScope::Tag => TasksBuilderError::UnknownDepTargetTagScope {
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
                    .is_ok_and(|id| id == project.id.as_str())
                    && dep_task_target.get_task_id()? == self.task.target.get_task_id()?
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
                                .aliases
                                .iter()
                                .any(|alias| alias.alias.as_str() == dep_project_id.as_str())
                                || project
                                    .dependencies
                                    .iter()
                                    .any(|pd| &pd.id == dep_project_id)
                        },
                    )
                {
                    project.dependencies.push(project_dep);
                }

                // The same target can be depended on multiple times (with
                // different args/env), but must always use the same type,
                // otherwise the graph would create competing edges
                let dep_type = normalize_type(task_dep.type_of);

                match dep_types.get(&task_dep.target) {
                    Some(other_type) if *other_type != dep_type => {
                        return Err(TasksBuilderError::ConflictingDepType {
                            dep: task_dep.target.to_owned(),
                            task: self.task.target.to_owned(),
                            current_type: *other_type,
                            other_type: dep_type,
                        }
                        .into());
                    }
                    Some(_) => {}
                    None => {
                        dep_types.insert(task_dep.target.to_owned(), dep_type);
                    }
                };

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
        // Cleanup and wait dependencies do not block the task, and their
        // result is never consumed, so many constraints do not apply
        let is_required = dep_config.type_of.is_required_type();

        // Do not depend on tasks that can fail
        if is_required && dep_task_options.allow_failure {
            return Err(TasksBuilderError::AllowFailureDepRequirement {
                dep: dep_task_target.to_owned(),
                task: self.task.target.to_owned(),
            }
            .into());
        }

        // Do not depend on tasks that can't run in CI
        if is_required
            && !dep_task_options.run_in_ci.is_enabled()
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
        match dep_config.type_of {
            // A cleanup dependency must run to completion after the task,
            // which is not possible when either side is persistent
            TaskDependencyType::Cleanup => {
                if dep_task_options.persistent {
                    return Err(TasksBuilderError::PersistentCleanupDepRequirement {
                        dep: dep_task_target.to_owned(),
                        task: self.task.target.to_owned(),
                    }
                    .into());
                }

                if self.task.options.persistent {
                    return Err(TasksBuilderError::PersistentCleanupTaskRequirement {
                        dep: dep_task_target.to_owned(),
                        task: self.task.target.to_owned(),
                    }
                    .into());
                }
            }
            // A wait dependency only waits for the dependency to start
            // running, which is the entire point of depending on a server
            TaskDependencyType::Wait => {}
            _ => {
                if dep_task_options.persistent && !self.task.options.persistent {
                    return Err(TasksBuilderError::PersistentDepRequirement {
                        dep: dep_task_target.to_owned(),
                        task: self.task.target.to_owned(),
                    }
                    .into());
                }
            }
        };

        // Add the dep if it has not already been
        let dep = TaskDependencyConfig {
            target: dep_task_target.to_owned(),
            cache_strategy: if is_required {
                dep_config.cache_strategy.or(Some(
                    if self.querent.query_task_has_outputs(dep_task_target) {
                        TaskDependencyCacheStrategy::Hash
                    } else {
                        TaskDependencyCacheStrategy::Ignored
                    },
                ))
            } else {
                // These dependencies do not contribute to the task's hash
                Some(TaskDependencyCacheStrategy::Ignored)
            },
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
    let Ok(dep_project_id) = task_dep.target.get_project_id().map(Id::raw) else {
        return None;
    };

    // Already a dependency, or references self
    if project_id == &dep_project_id || already_exists(&dep_project_id) {
        return None;
    }

    debug!(
        project_id = project_id.as_str(),
        dep_id = dep_project_id.as_str(),
        task_target = task_dep.target.as_str(),
        "Marking arbitrary project as an implicit dependency because of a task dependency"
    );

    Some(ProjectDependencyConfig {
        scope: if root_project_id.is_some_and(|id| id == &dep_project_id) {
            DependencyScope::Root
        } else {
            DependencyScope::Build
        },
        id: dep_project_id,
        source: DependencySource::Implicit,
        via: Some(format!("task {}", task_dep.target)),
    })
}
