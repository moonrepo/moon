use crate::output_archiver::OutputArchiver;
use crate::output_hydrater::{HydrateFrom, OutputHydrater};
use crate::run_state::RunTaskState;
use crate::task_runner_error::TaskRunnerError;
use moon_action::{ActionNode, ActionStatus};
use moon_action_context::{ActionContext, TargetState};
use moon_cache::CacheItem;
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

    archiver: OutputArchiver<'task>,
    cache: CacheItem<RunTaskState>,
    hydrater: OutputHydrater<'task>,
}

impl<'task> TaskRunner<'task> {
    pub fn new(
        node: &'task ActionNode,
        project: &'task Project,
        task: &'task Task,
        workspace: &'task Workspace,
    ) -> miette::Result<Self> {
        let mut cache = workspace
            .cache_engine
            .state
            .load_target_state::<RunTaskState>(&task.target)?;

        if cache.data.target.is_empty() {
            cache.data.target = task.target.to_string();
        }

        Ok(Self {
            cache,
            archiver: OutputArchiver {
                project_config: &project.config,
                task,
                workspace,
            },
            hydrater: OutputHydrater { task, workspace },
            node,
            project,
            task,
            workspace,
        })
    }

    pub async fn is_cached(&self, hash: &str) -> miette::Result<Option<HydrateFrom>> {
        let cache_engine = &self.workspace.cache_engine;

        // If hash is the same as the previous build, we can simply abort!
        // However, ensure the outputs also exist, otherwise we should hydrate
        if self.cache.data.exit_code == 0
            && self.cache.data.hash == hash
            && self.archiver.has_outputs_been_created(true)?
        {
            return Ok(Some(HydrateFrom::PreviousOutput));
        }

        if !cache_engine.get_mode().is_readable() {
            return Ok(None);
        }

        // Check if archive exists in moonbase (remote storage) by querying the artifacts
        // endpoint. This only checks that the database record exists!
        if let Some(moonbase) = &self.workspace.session {
            if let Some(_) = moonbase.read_artifact(hash).await? {
                return Ok(Some(HydrateFrom::RemoteCache));
            }
        }

        // Check to see if a build with the provided hash has been cached locally.
        // We only check for the archive, as the manifest is purely for local debugging!
        if cache_engine.hash.get_archive_path(hash).exists() {
            return Ok(Some(HydrateFrom::LocalCache));
        }

        Ok(None)
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
            if let Some(from) = self.is_cached(&hash).await? {
                self.hydrater.hydrate(&hash, from).await?;

                context.set_target_state(&self.task.target, TargetState::Completed(hash));

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
