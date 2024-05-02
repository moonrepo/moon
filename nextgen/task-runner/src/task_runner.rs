use crate::command_builder::CommandBuilder;
use crate::command_executor::CommandExecutor;
use crate::output_archiver::OutputArchiver;
use crate::output_hydrater::{HydrateFrom, OutputHydrater};
use crate::run_state::RunTaskState;
use crate::task_runner_error::TaskRunnerError;
use moon_action::{ActionNode, ActionStatus, Attempt};
use moon_action_context::{ActionContext, TargetState};
use moon_cache::CacheItem;
use moon_console::Console;
use moon_platform::PlatformManager;
use moon_project::Project;
use moon_task::Task;
use moon_task_hasher::TaskHasher;
use moon_time::now_millis;
use moon_workspace::Workspace;
use starbase_utils::fs;
use std::collections::BTreeMap;
use std::sync::Arc;
use tracing::{debug, trace};

pub struct TaskRunResult {
    pub attempts: Vec<Attempt>,
    pub status: ActionStatus,
}

impl TaskRunResult {
    pub fn new(status: ActionStatus) -> Self {
        Self {
            attempts: vec![],
            status,
        }
    }
}

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
        workspace: &'task Workspace,
        project: &'task Project,
        task: &'task Task,
        node: &'task ActionNode,
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

    pub async fn run(
        &mut self,
        context: &ActionContext,
        console: Arc<Console>,
    ) -> miette::Result<TaskRunResult> {
        let target = &self.task.target;

        // If a dependency has failed or been skipped, we should skip this task
        if !self.is_dependencies_complete(context)? {
            context.set_target_state(target, TargetState::Skipped);

            return Ok(TaskRunResult::new(ActionStatus::Skipped));
        }

        // Generate a unique hash so we can check the cache
        let hash = self.generate_hash(context).await?;

        // Exit early if this build has already been cached/hashed
        if self.is_cache_enabled() {
            if let Some(from) = self.is_cached(&hash).await? {
                self.hydrater.hydrate(&hash, from).await?;

                self.persist_cache(&hash)?;

                context.set_target_state(target, TargetState::Completed(hash));

                return Ok(TaskRunResult::new(match from {
                    HydrateFrom::RemoteCache => ActionStatus::CachedFromRemote,
                    _ => ActionStatus::Cached,
                }));
            }
        } else {
            debug!(
                task = self.task.target.as_str(),
                hash = &hash,
                "Caching is disabled for task, will attempt to run a command"
            );

            // We must give this task a fake hash for it to be considered complete
            // for other tasks! This case triggers for noop or cache disabled tasks.
            context.set_target_state(target, TargetState::Passthrough);
        }

        // Otherwise build and execute the command as a child process
        match self
            .execute_command(&hash, context, Arc::clone(&console))
            .await
        {
            Ok(attempts) => {
                self.archiver.archive(&hash).await?;

                self.persist_cache(&hash)?;

                context.set_target_state(target, TargetState::Completed(hash));

                Ok(TaskRunResult {
                    status: ActionStatus::Passed,
                    attempts,
                })
            }
            Err(error) => {
                self.persist_cache(&hash)?;

                context.set_target_state(target, TargetState::Failed);

                Err(error)
            }
        }
    }

    async fn is_cached(&self, hash: &str) -> miette::Result<Option<HydrateFrom>> {
        let cache_engine = &self.workspace.cache_engine;

        debug!(
            task = self.task.target.as_str(),
            hash, "Checking if task has been cached using hash"
        );

        // If hash is the same as the previous build, we can simply abort!
        // However, ensure the outputs also exist, otherwise we should hydrate
        if self.cache.data.exit_code == 0
            && self.cache.data.hash == hash
            && self.archiver.has_outputs_been_created(true)?
        {
            debug!(
                task = self.task.target.as_str(),
                hash, "Hash matches previous run, reusing existing outputs"
            );

            return Ok(Some(HydrateFrom::PreviousOutput));
        }

        if !cache_engine.get_mode().is_readable() {
            debug!(
                task = self.task.target.as_str(),
                hash, "Cache is not readable, continuing run"
            );

            return Ok(None);
        }

        // Check to see if a build with the provided hash has been cached locally.
        // We only check for the archive, as the manifest is purely for local debugging!
        let archive_file = cache_engine.hash.get_archive_path(hash);

        if archive_file.exists() {
            debug!(
                task = self.task.target.as_str(),
                hash,
                archive_file = ?archive_file,
                "Cache hit in local cache, will reuse existing archive"
            );

            return Ok(Some(HydrateFrom::LocalCache));
        }

        // Check if archive exists in moonbase (remote storage) by querying the artifacts
        // endpoint. This only checks that the database record exists!
        if let Some(moonbase) = &self.workspace.session {
            if let Some((artifact, _)) = moonbase.read_artifact(hash).await? {
                debug!(
                    task = self.task.target.as_str(),
                    hash,
                    artifact_id = artifact.id,
                    "Cache hit in remote cache, will attempt to download the archive"
                );

                return Ok(Some(HydrateFrom::RemoteCache));
            }
        }

        debug!(
            task = self.task.target.as_str(),
            hash, "Cache miss, continuing run"
        );

        Ok(None)
    }

    fn is_cache_enabled(&self) -> bool {
        // If the VCS root does not exist (like in a Docker container),
        // we should avoid failing and simply disable caching
        self.task.options.cache && self.workspace.vcs.is_enabled()
    }

    fn is_dependencies_complete(&self, context: &ActionContext) -> miette::Result<bool> {
        if self.task.deps.is_empty() {
            return Ok(true);
        }

        for dep in &self.task.deps {
            if let Some(dep_state) = context.target_states.get(&dep.target) {
                if dep_state.get().is_complete() {
                    continue;
                }

                debug!(
                    task = self.task.target.as_str(),
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

    async fn generate_hash(&self, context: &ActionContext) -> miette::Result<String> {
        debug!(
            task = self.task.target.as_str(),
            "Generating a unique hash for this task"
        );

        let mut hasher = self
            .workspace
            .cache_engine
            .hash
            .create_hasher(self.node.label());

        // Hash common fields
        trace!(
            task = self.task.target.as_str(),
            "Including common task related fields in the hash"
        );

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
        trace!(
            task = self.task.target.as_str(),
            platform = ?self.task.platform,
            "Including platform specific fields in the hash"
        );

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

        debug!(
            task = self.task.target.as_str(),
            hash = &hash,
            "Generated a unique hash"
        );

        Ok(hash)
    }

    async fn execute_command(
        &mut self,
        hash: &str,
        context: &ActionContext,
        console: Arc<Console>,
    ) -> miette::Result<Vec<Attempt>> {
        if self.task.is_no_op() {
            return Ok(vec![]);
        }

        // Build the command from the current task
        let command = CommandBuilder::new(&self.workspace, &self.project, &self.task, &self.node)
            .build(context)
            .await?;

        // Execute the command and gather all attempts made
        let executor = CommandExecutor::new(&self.task, &self.node, command);

        let attempts = if let Some(mutex_name) = &self.task.options.mutex {
            let mutex = context.get_or_create_mutex(mutex_name);
            let _guard = mutex.lock().await;

            // This execution is required within this block so that the
            // guard above isn't immediately dropped!
            executor.execute(hash, context, console).await?
        } else {
            executor.execute(hash, context, console).await?
        };

        // Persist the last attempt state
        if let Some(last_attempt) = attempts.last() {
            let state_dir = self
                .workspace
                .cache_engine
                .state
                .get_target_dir(&self.task.target);

            self.cache.data.exit_code = last_attempt.exit_code.unwrap_or(-1);

            if let Some(log) = &last_attempt.stderr {
                let _ = fs::write_file(state_dir.join("stderr.log"), log.as_bytes());
            }

            if let Some(log) = &last_attempt.stdout {
                let _ = fs::write_file(state_dir.join("stdout.log"), log.as_bytes());
            }
        }

        Ok(vec![])
    }

    fn persist_cache(&mut self, hash: &str) -> miette::Result<()> {
        self.cache.data.hash = hash.to_owned();
        self.cache.data.last_run_time = now_millis();
        self.cache.save()?;

        Ok(())
    }
}
