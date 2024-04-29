use crate::output_hydrater::{HydrateFrom, OutputHydrater};
use crate::task_runner_error::TaskRunnerError;
use moon_action::{ActionNode, ActionStatus};
use moon_action_context::{ActionContext, TargetState};
use moon_platform::PlatformManager;
use moon_project::Project;
use moon_task::Task;
use moon_task_hasher::TaskHasher;
use moon_workspace::Workspace;
use std::collections::BTreeMap;
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

                debug!(
                    target = self.task.target.as_str(),
                    dependency = dep.target.as_str(),
                    "Task dependency has failed or has been skipped, skipping this task",
                );

                return Ok(false);
            } else {
                return Err(TaskRunnerError::MissingDependencyHash {
                    dep_target: dep.target.id.to_owned(),
                    target: self.task.target.id.to_owned(),
                }
                .into());
            }
        }

        return Ok(true);
    }

    pub async fn generate_hash(&self, context: &ActionContext) -> miette::Result<String> {
        let mut hasher = self
            .workspace
            .cache_engine
            .hash
            .create_hasher(self.node.label());

        // Hash common fields
        let mut task_hasher = TaskHasher::new(
            self.project,
            self.task,
            &self.workspace.vcs,
            &self.workspace.root,
            &self.workspace.config.hasher,
        );

        if context.should_inherit_args(&self.task.target) {
            task_hasher.hash_args(&context.passthrough_args);
        }

        task_hasher.hash_deps({
            let mut deps = BTreeMap::default();

            for dep in &self.task.deps {
                if let Some(entry) = context.target_states.get(&dep.target) {
                    match entry.get() {
                        TargetState::Completed(hash) => {
                            deps.insert(&dep.target, hash.clone());
                        }
                        TargetState::Passthrough => {
                            deps.insert(&dep.target, "passthrough".into());
                        }
                        _ => {}
                    };
                }
            }

            deps
        });

        task_hasher.hash_inputs().await?;

        hasher.hash_content(task_hasher.hash())?;

        // Hash platform fields
        PlatformManager::read()
            .get(self.task.platform)?
            .hash_run_target(
                self.project,
                self.node.get_runtime(),
                &mut hasher,
                &self.workspace.config.hasher,
            )
            .await?;

        let hash = self.workspace.cache_engine.hash.save_manifest(hasher)?;

        Ok(hash)
    }

    pub async fn hydrate(&self, from: HydrateFrom) -> miette::Result<()> {
        OutputHydrater {
            cache_engine: &self.workspace.cache_engine,
            task: self.task,
            workspace_root: &self.workspace.root,
        }
        .hydrate(&self.hash, from)
        .await?;

        Ok(())
    }

    pub async fn run(&self, context: &ActionContext) -> miette::Result<ActionStatus> {
        // If a dependency has failed or been skipped, we should skip this task
        if !self.is_dependencies_complete(context)? {
            context.set_target_state(&self.task.target, TargetState::Skipped);

            return Ok(ActionStatus::Skipped);
        }

        // Generate a unique hash so we can check the cache
        let hash = self.generate_hash(context).await?;

        // Exit early if this build has already been cached/hashed
        if self.is_cache_enabled() {
            if let Some(from) = self.is_cached() {
                context.set_target_state(&self.task.target, TargetState::Completed(hash));

                self.hydrate(from).await?;

                return Ok(match from {
                    HydrateFrom::RemoteCache => ActionStatus::CachedFromRemote,
                    _ => ActionStatus::Cached,
                });
            }
        } else {
            // We must give this task a fake hash for it to be considered complete
            // for other tasks! This case triggers for noop or cache disabled tasks.
            context.set_target_state(&self.task.target, TargetState::Passthrough);
        }

        Ok(ActionStatus::Passed)
    }
}
