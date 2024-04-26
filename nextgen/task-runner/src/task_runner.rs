use crate::output_hydrater::{HydrateFrom, OutputHydrater};
use moon_action::{ActionNode, ActionStatus};
use moon_action_context::{ActionContext, TargetState};
use moon_project::Project;
use moon_task::Task;
use moon_workspace::Workspace;
use tracing::debug;

pub struct TaskRunner<'task> {
    node: &'task ActionNode,
    project: &'task Project,
    task: &'task Task,
    workspace: &'task Workspace,

    hash: String,
}

impl<'task> TaskRunner<'task> {
    pub fn is_cached(&self) -> Option<HydrateFrom> {
        None
    }

    pub fn is_cache_enabled(&self) -> bool {
        // If the VCS root does not exist (like in a Docker container),
        // we should avoid failing and simply disable caching
        self.task.options.cache && self.workspace.vcs.is_enabled()
    }

    pub fn is_dependencies_complete(&self, context: &ActionContext) -> miette::Result<bool> {
        if self.task.deps.is_empty() {
            return Ok(true);
        }

        for dep in &self.task.deps {
            if let Some(dep_state) = context.target_states.get(&dep.target) {
                if dep_state.get().is_complete() {
                    continue;
                }

                context.set_target_state(&self.task.target, TargetState::Skipped);

                debug!(
                    target = self.task.target.as_str(),
                    dependency = dep.target.as_str(),
                    "Task dependency has failed or has been skipped, skipping this task",
                );

                return Ok(false);
            }
        }

        return Ok(true);
    }

    pub async fn hydrate(&self, from: HydrateFrom) -> miette::Result<ActionStatus> {
        OutputHydrater {
            cache_engine: &self.workspace.cache_engine,
            task: self.task,
            workspace_root: &self.workspace.root,
        }
        .hydrate(&self.hash, from)
        .await?;

        Ok(match from {
            HydrateFrom::RemoteCache => ActionStatus::CachedFromRemote,
            _ => ActionStatus::Cached,
        })
    }

    pub async fn run(&self, context: &ActionContext) -> miette::Result<ActionStatus> {
        // If a dependency has failed or been skipped, we should skip this task
        if !self.is_dependencies_complete(context)? {
            return Ok(ActionStatus::Skipped);
        }

        // Exit early if this build has already been cached/hashed
        if self.is_cache_enabled() {
            if let Some(from) = self.is_cached() {
                return Ok(self.hydrate(from).await?);
            }
        }

        Ok(ActionStatus::Passed)
    }
}
