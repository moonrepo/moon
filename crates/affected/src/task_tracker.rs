use crate::affected::*;
use moon_common::Id;
use moon_common::path::WorkspaceRelativePathBuf;
use moon_env_var::GlobalEnvBag;
use moon_task::{Target, Task, TaskOptionRunInCI};
use moon_workspace_graph::{GraphConnections, WorkspaceGraph};
use rustc_hash::{FxHashMap, FxHashSet};
use starbase_utils::fs;
use std::sync::Arc;
use tracing::trace;

pub struct TaskTracker {
    pub changed_files: Arc<FxHashSet<WorkspaceRelativePathBuf>>,
    pub ci: bool,
    pub downstream: DownstreamScope,
    pub task: Arc<Task>,
    pub tracked: FxHashMap<Target, FxHashSet<AffectedBy>>,
    pub tracked_projects: FxHashMap<Id, FxHashSet<AffectedBy>>,
    pub upstream: UpstreamScope,
    pub workspace_graph: Arc<WorkspaceGraph>,
}

impl TaskTracker {
    pub async fn track(mut self) -> miette::Result<Self> {
        let task = Arc::clone(&self.task);

        if let Some(affected) = self.is_task_affected(&task)? {
            self.mark_task_affected(&task, affected)?;
        }

        Ok(self)
    }

    pub fn is_task_affected(&self, task: &Task) -> miette::Result<Option<AffectedBy>> {
        // Special CI handling
        match (self.ci, &task.options.run_in_ci) {
            (true, TaskOptionRunInCI::Always) => {
                return Ok(Some(AffectedBy::AlwaysAffected));
            }
            (true, TaskOptionRunInCI::Enabled(false))
            | (true, TaskOptionRunInCI::Skip)
            | (false, TaskOptionRunInCI::Only) => {
                return Ok(None);
            }
            _ => {}
        };

        // Never affected
        if task.state.empty_inputs {
            return Ok(None);
        }

        // By env vars
        if !task.input_env.is_empty() {
            let bag = GlobalEnvBag::instance();

            for var_name in &task.input_env {
                if let Some(var) = bag.get(var_name)
                    && !var.is_empty()
                {
                    return Ok(Some(AffectedBy::EnvironmentVariable(var_name.to_owned())));
                }
            }
        }

        // By files
        let globset = task.create_globset()?;

        for file in self.changed_files.iter() {
            let affected = if let Some(params) = task.input_files.get(file) {
                match &params.content {
                    Some(matcher) => {
                        let abs_file = file.to_logical_path(&self.workspace_graph.root);

                        if abs_file.exists() {
                            matcher.is_match(&fs::read_file(abs_file)?)
                        } else {
                            false
                        }
                    }
                    None => true,
                }
            } else {
                globset.matches(file.as_str())
            };

            if affected {
                return Ok(Some(AffectedBy::ChangedFile(file.to_owned())));
            }
        }

        Ok(None)
    }

    pub fn mark_task_affected(&mut self, task: &Task, affected: AffectedBy) -> miette::Result<()> {
        if affected == AffectedBy::AlreadyMarked {
            // May have been already marked through an indirect dep,
            // but that doesn't mean its own deps have been checked!
            self.track_task_dependencies(task, 0, &mut FxHashSet::default())?;
            self.track_task_dependents(task, 0, &mut FxHashSet::default())?;

            return Ok(());
        }

        trace!(
            task_target = task.target.as_str(),
            "Marking task as affected"
        );

        self.tracked
            .entry(task.target.clone())
            .or_default()
            .insert(affected);

        self.track_task_dependencies(task, 0, &mut FxHashSet::default())?;
        self.track_task_dependents(task, 0, &mut FxHashSet::default())?;

        if let Ok(project_id) = task.target.get_project_id() {
            self.tracked_projects
                .entry(project_id.to_owned())
                .or_default()
                .insert(AffectedBy::Task(task.target.clone()));
        }

        Ok(())
    }

    fn track_task_dependencies(
        &mut self,
        task: &Task,
        depth: u16,
        cycle: &mut FxHashSet<Target>,
    ) -> miette::Result<()> {
        if cycle.contains(&task.target) {
            return Ok(());
        }

        cycle.insert(task.target.clone());

        if self.upstream == UpstreamScope::None {
            trace!(
                task_target = task.target.as_str(),
                "Not tracking task dependencies as upstream scope is none"
            );

            return Ok(());
        }

        if depth == 0 {
            if self.upstream == UpstreamScope::Direct {
                trace!(
                    task_target = task.target.as_str(),
                    "Tracking direct task dependencies"
                );
            } else {
                trace!(
                    task_target = task.target.as_str(),
                    "Tracking deep task dependencies"
                );
            }
        }

        for dep_config in &task.deps {
            self.tracked
                .entry(dep_config.target.clone())
                .or_default()
                .insert(AffectedBy::DownstreamTask(task.target.clone()));

            if depth == 0 && self.upstream == UpstreamScope::Direct {
                continue;
            }

            let dep_task = self.workspace_graph.get_task(&dep_config.target)?;

            self.track_task_dependencies(&dep_task, depth + 1, cycle)?;
        }

        Ok(())
    }

    fn track_task_dependents(
        &mut self,
        task: &Task,
        depth: u16,
        cycle: &mut FxHashSet<Target>,
    ) -> miette::Result<()> {
        if cycle.contains(&task.target) {
            return Ok(());
        }

        cycle.insert(task.target.clone());

        if self.downstream == DownstreamScope::None {
            trace!(
                task_target = task.target.as_str(),
                "Not tracking task dependents as downstream scope is none"
            );

            return Ok(());
        }

        if depth == 0 {
            if self.downstream == DownstreamScope::Direct {
                trace!(
                    task_target = task.target.as_str(),
                    "Tracking direct task dependents"
                );
            } else {
                trace!(
                    task_target = task.target.as_str(),
                    "Tracking deep task dependents"
                );
            }
        }

        for dep_target in self.workspace_graph.tasks.dependents_of(task) {
            self.tracked
                .entry(dep_target.clone())
                .or_default()
                .insert(AffectedBy::UpstreamTask(task.target.clone()));

            if depth == 0 && self.downstream == DownstreamScope::Direct {
                continue;
            }

            let dep_task = self.workspace_graph.get_task(&dep_target)?;

            self.track_task_dependents(&dep_task, depth + 1, cycle)?;
        }

        Ok(())
    }
}
