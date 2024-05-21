use crate::command_builder::CommandBuilder;
use crate::command_executor::CommandExecutor;
use crate::output_archiver::OutputArchiver;
use crate::output_hydrater::{HydrateFrom, OutputHydrater};
use crate::run_state::RunTaskState;
use crate::task_runner_error::TaskRunnerError;
use moon_action::{ActionNode, ActionStatus, Attempt, AttemptType};
use moon_action_context::{ActionContext, TargetState};
use moon_cache::CacheItem;
use moon_console::{Console, TaskReportState};
use moon_platform::PlatformManager;
use moon_process::ProcessError;
use moon_project::Project;
use moon_task::Task;
use moon_task_hasher::TaskHasher;
use moon_time::now_millis;
use moon_workspace::Workspace;
use starbase_utils::fs;
use std::collections::BTreeMap;
use std::mem;
use std::sync::Arc;
use tracing::{debug, trace};

pub struct TaskRunResult {
    pub attempts: Vec<Attempt>,
    pub hash: Option<String>,
}

pub struct TaskRunner<'task> {
    project: &'task Project,
    pub task: &'task Task,
    workspace: &'task Workspace,
    platform_manager: &'task PlatformManager,

    archiver: OutputArchiver<'task>,
    console: Arc<Console>,
    hydrater: OutputHydrater<'task>,

    // Public for testing
    pub attempts: Vec<Attempt>,
    pub cache: CacheItem<RunTaskState>,
}

impl<'task> TaskRunner<'task> {
    pub fn new(
        workspace: &'task Workspace,
        project: &'task Project,
        task: &'task Task,
        console: Arc<Console>,
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
            console,
            archiver: OutputArchiver {
                project_config: &project.config,
                task,
                workspace,
            },
            hydrater: OutputHydrater { task, workspace },
            platform_manager: PlatformManager::read(),
            project,
            task,
            workspace,
            attempts: vec![],
        })
    }

    pub fn set_platform_manager(&mut self, manager: &'task PlatformManager) {
        self.platform_manager = manager;
    }

    pub async fn run(
        &mut self,
        context: &ActionContext,
        node: &ActionNode,
    ) -> miette::Result<Option<String>> {
        // If a dependency has failed or been skipped, we should skip this task
        if !self.is_dependencies_complete(context)? {
            self.skip(context)?;

            return Ok(None);
        }

        if self.is_cache_enabled() {
            let hash = self.generate_hash(context, node).await?;

            // Exit early if this build has already been cached/hashed
            if self.hydrate(context, &hash).await? {
                return Ok(Some(hash));
            }

            // Otherwise build and execute the command as a child process
            self.execute(context, node, Some(&hash)).await?;

            // If we created outputs, archive them into the cache
            self.archive(&hash).await?;

            return Ok(Some(hash));
        }

        debug!(
            task = self.task.target.as_str(),
            "Caching is disabled for task, will not generate a hash, and will attempt to run a command as normal"
        );

        // Mark it as passthrough early so that other tasks that depend on it don't fail
        // context.set_target_state(&self.task.target, TargetState::Passthrough);

        // Build and execute the command as a child process
        self.execute(context, node, None).await?;

        Ok(None)
    }

    pub async fn run_and_persist(
        &mut self,
        context: &ActionContext,
        node: &ActionNode,
    ) -> miette::Result<TaskRunResult> {
        let result = self.run(context, node).await;

        if let Some(last_attempt) = Attempt::get_last_execution(&self.attempts) {
            self.cache.data.exit_code = last_attempt.get_exit_code();
            self.save_logs(last_attempt)?;
        }

        self.cache.data.last_run_time = now_millis();
        self.cache.save()?;

        // We lose the attempt state here, is that ok?
        let mut state = TaskReportState::default();

        match result {
            Ok(maybe_hash) => {
                state.hash = maybe_hash.clone();

                self.console.reporter.on_task_completed(
                    &self.task.target,
                    &self.attempts,
                    &state,
                    None,
                )?;

                Ok(TaskRunResult {
                    attempts: mem::take(&mut self.attempts),
                    hash: maybe_hash,
                })
            }
            Err(error) => {
                self.console.reporter.on_task_completed(
                    &self.task.target,
                    &self.attempts,
                    &state,
                    Some(&error),
                )?;

                Err(error)
            }
        }
    }

    pub async fn is_cached(&mut self, hash: &str) -> miette::Result<Option<HydrateFrom>> {
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

        // Set this *after* we checked the previous cache
        self.cache.data.hash = hash.to_owned();

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

        Ok(true)
    }

    pub async fn generate_hash(
        &mut self,
        context: &ActionContext,
        node: &ActionNode,
    ) -> miette::Result<String> {
        debug!(
            task = self.task.target.as_str(),
            "Generating a unique hash for this task"
        );

        let mut attempt = Attempt::new(AttemptType::HashGeneration);
        let mut hasher = self.workspace.cache_engine.hash.create_hasher(node.label());

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
                        TargetState::Passed(hash) => {
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

        self.platform_manager
            .get(self.task.platform)?
            .hash_run_target(
                self.project,
                node.get_runtime(),
                &mut hasher,
                &self.workspace.config.hasher,
            )
            .await?;

        let hash = self.workspace.cache_engine.hash.save_manifest(hasher)?;

        attempt.finish(ActionStatus::Passed);
        self.attempts.push(attempt);

        debug!(
            task = self.task.target.as_str(),
            hash = &hash,
            "Generated a unique hash"
        );

        Ok(hash)
    }

    pub async fn execute(
        &mut self,
        context: &ActionContext,
        node: &ActionNode,
        hash: Option<&str>,
    ) -> miette::Result<()> {
        if self.task.is_no_op() {
            self.attempts.push(Attempt::new_finished(
                AttemptType::NoOperation,
                ActionStatus::Passed,
            ));

            return Ok(());
        }

        // Build the command from the current task
        let mut builder = CommandBuilder::new(self.workspace, self.project, self.task, node);
        builder.set_platform_manager(&self.platform_manager);

        let command = builder.build(context).await?;

        // Execute the command and gather all attempts made
        let executor = CommandExecutor::new(
            self.workspace,
            self.project,
            self.task,
            node,
            self.console.clone(),
            command,
        );

        let result = if let Some(mutex_name) = &self.task.options.mutex {
            let mut attempt = Attempt::new(AttemptType::MutexAcquisition);
            let mutex = context.get_or_create_mutex(mutex_name);
            let _guard = mutex.lock().await;

            attempt.finish(ActionStatus::Passed);
            self.attempts.push(attempt);

            // This execution is required within this block so that the
            // guard above isn't immediately dropped!
            executor.execute(context, hash).await?
        } else {
            executor.execute(context, hash).await?
        };

        // Update the action state based on the result
        context.set_target_state(&self.task.target, result.run_state);

        // Extract the attempts from the result
        self.attempts.extend(result.attempts);

        // If the execution as a whole failed, return the error.
        // We do this here instead of in `execute` so that we can
        // capture the attempts and report them.
        if let Some(result_error) = result.error {
            return Err(result_error);
        }

        // If our last task execution was a failure, return a hard error
        if let Some(last_attempt) = Attempt::get_last_failed_execution(&self.attempts) {
            if last_attempt.has_failed() {
                return Err(TaskRunnerError::RunFailed {
                    target: self.task.target.to_string(),
                    error: ProcessError::ExitNonZero {
                        bin: self.task.command.clone(),
                        code: last_attempt.get_exit_code(),
                    },
                }
                .into());
            }
        }

        Ok(())
    }

    pub fn skip(&mut self, context: &ActionContext) -> miette::Result<()> {
        self.attempts.push(Attempt::new_finished(
            AttemptType::TaskExecution,
            ActionStatus::Skipped,
        ));

        context.set_target_state(&self.task.target, TargetState::Skipped);

        Ok(())
    }

    pub async fn archive(&mut self, hash: &str) -> miette::Result<bool> {
        let mut attempt = Attempt::new(AttemptType::ArchiveCreation);

        let archived = match self.archiver.archive(hash).await? {
            Some(_) => {
                attempt.finish(ActionStatus::Passed);
                true
            }
            None => {
                attempt.finish(ActionStatus::Skipped);
                false
            }
        };

        self.attempts.push(attempt);

        Ok(archived)
    }

    async fn hydrate(&mut self, context: &ActionContext, hash: &str) -> miette::Result<bool> {
        let mut attempt = Attempt::new(AttemptType::OutputHydration);

        let hydrated = match self.is_cached(hash).await? {
            Some(from) => {
                self.hydrater.hydrate(hash, from).await?;
                self.load_logs(&mut attempt)?;

                attempt.finish(match from {
                    HydrateFrom::RemoteCache => ActionStatus::CachedFromRemote,
                    _ => ActionStatus::Cached,
                });

                context.set_target_state(&self.task.target, TargetState::Passed(hash.to_owned()));

                true
            }
            None => {
                attempt.finish(ActionStatus::Skipped);

                false
            }
        };

        self.attempts.push(attempt);

        Ok(hydrated)
    }

    fn load_logs(&self, attempt: &mut Attempt) -> miette::Result<()> {
        let state_dir = self
            .workspace
            .cache_engine
            .state
            .get_target_dir(&self.task.target);
        let err_path = state_dir.join("stderr.log");
        let out_path = state_dir.join("stdout.log");

        if let Some(execution) = &mut attempt.execution {
            if err_path.exists() {
                execution.set_stderr(fs::read_file(err_path)?);
            }

            if out_path.exists() {
                execution.set_stdout(fs::read_file(out_path)?);
            }
        }

        Ok(())
    }

    fn save_logs(&self, attempt: &Attempt) -> miette::Result<()> {
        let state_dir = self
            .workspace
            .cache_engine
            .state
            .get_target_dir(&self.task.target);

        if let Some(execution) = &attempt.execution {
            if let Some(log) = &execution.stderr {
                fs::write_file(state_dir.join("stderr.log"), log.as_bytes())?;
            }

            if let Some(log) = &execution.stdout {
                fs::write_file(state_dir.join("stdout.log"), log.as_bytes())?;
            }
        }

        Ok(())
    }
}
