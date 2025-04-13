use crate::command_builder::CommandBuilder;
use crate::command_executor::CommandExecutor;
use crate::output_archiver::OutputArchiver;
use crate::output_hydrater::{HydrateFrom, OutputHydrater};
use crate::run_state::*;
use crate::task_runner_error::TaskRunnerError;
use moon_action::{ActionNode, ActionStatus, Operation, OperationList, OperationMeta};
use moon_action_context::{ActionContext, TargetState};
use moon_app_context::AppContext;
use moon_cache::CacheItem;
use moon_console::TaskReportItem;
use moon_pdk_api::HashTaskContentsInput;
use moon_platform::PlatformManager;
use moon_process::ProcessError;
use moon_project::Project;
use moon_remote::{ActionState, Digest, RemoteService};
use moon_task::Task;
use moon_task_hasher::TaskHasher;
use moon_time::{is_stale, now_millis};
use starbase_utils::fs;
use std::collections::BTreeMap;
use std::time::SystemTime;
use tracing::{debug, instrument, trace};

#[derive(Debug)]
pub struct TaskRunResult {
    pub hash: Option<String>,
    pub error: Option<miette::Report>,
    pub operations: OperationList,
}

pub struct TaskRunner<'task> {
    app: &'task AppContext,
    project: &'task Project,
    pub task: &'task Task,
    platform_manager: &'task PlatformManager,

    archiver: OutputArchiver<'task>,
    hydrater: OutputHydrater<'task>,

    // Public for testing
    pub cache: CacheItem<TaskRunCacheState>,
    pub operations: OperationList,
    pub remote_state: Option<ActionState<'task>>,
    pub report_item: TaskReportItem,
    pub target_state: Option<TargetState>,
}

impl<'task> TaskRunner<'task> {
    pub fn new(
        app: &'task AppContext,
        project: &'task Project,
        task: &'task Task,
    ) -> miette::Result<Self> {
        debug!(
            task_target = task.target.as_str(),
            "Creating a task runner for target"
        );

        let mut cache = app
            .cache_engine
            .state
            .load_target_state::<TaskRunCacheState>(&task.target)?;

        if cache.data.target.is_empty() {
            cache.data.target = task.target.to_string();
        }

        Ok(Self {
            cache,
            archiver: OutputArchiver { app, project, task },
            hydrater: OutputHydrater { app, task },
            platform_manager: PlatformManager::read(),
            project,
            remote_state: None,
            report_item: TaskReportItem {
                output_style: task.options.output_style,
                ..Default::default()
            },
            target_state: None,
            task,
            app,
            operations: OperationList::default(),
        })
    }

    pub fn set_platform_manager(&mut self, manager: &'task PlatformManager) {
        self.platform_manager = manager;
    }

    async fn internal_run(
        &mut self,
        context: &ActionContext,
        node: &ActionNode,
    ) -> miette::Result<Option<String>> {
        // If a dependency has failed or been skipped, we should skip this task
        if !self.is_dependencies_complete(context)? {
            self.skip()?;

            return Ok(None);
        }

        // If cache is enabled, then generate a hash and manage outputs
        if self.is_cache_enabled() {
            debug!(
                task_target = self.task.target.as_str(),
                "Caching is enabled for task, will generate a hash and manage outputs"
            );

            let hash = self.generate_hash(context, node).await?;

            // Exit early if this build has already been cached/hashed
            if self.hydrate(&hash).await? {
                return Ok(Some(hash));
            }

            // Otherwise build and execute the command as a child process
            self.execute(context, node).await?;

            // If we created outputs, archive them into the cache
            self.archive(&hash).await?;

            return Ok(Some(hash));
        }

        debug!(
            task_target = self.task.target.as_str(),
            "Caching is disabled for task, will not generate a hash, and will attempt to run a command as normal"
        );

        // Otherwise build and execute the command as a child process
        self.execute(context, node).await?;

        Ok(None)
    }

    #[instrument(skip(self, context))]
    pub async fn run(
        &mut self,
        context: &ActionContext,
        node: &ActionNode,
    ) -> miette::Result<TaskRunResult> {
        self.report_item.output_prefix = Some(context.get_target_prefix(&self.task.target));

        let result = self.internal_run(context, node).await;

        self.cache.data.last_run_time = now_millis();
        self.cache.save()?;

        match result {
            Ok(maybe_hash) => {
                context.set_target_state(
                    &self.task.target,
                    self.target_state.take().unwrap_or(TargetState::Passthrough),
                );

                self.report_item.hash = maybe_hash.clone();

                self.app.console.on_task_completed(
                    &self.task.target,
                    &self.operations,
                    &self.report_item,
                    None,
                )?;

                Ok(TaskRunResult {
                    error: None,
                    hash: maybe_hash,
                    operations: self.operations.take(),
                })
            }
            Err(error) => {
                context.set_target_state(
                    &self.task.target,
                    self.target_state.take().unwrap_or(TargetState::Failed),
                );

                self.inject_failed_task_execution(Some(&error))?;

                self.app.console.on_task_completed(
                    &self.task.target,
                    &self.operations,
                    &self.report_item,
                    Some(&error),
                )?;

                Ok(TaskRunResult {
                    error: Some(error),
                    hash: None,
                    operations: self.operations.take(),
                })
            }
        }
    }

    #[cfg(debug_assertions)]
    pub async fn run_with_panic(
        &mut self,
        context: &ActionContext,
        node: &ActionNode,
    ) -> miette::Result<TaskRunResult> {
        let result = self.run(context, node).await?;

        if let Some(error) = result.error {
            panic!("{}", error.to_string());
        }

        Ok(result)
    }

    #[instrument(skip(self))]
    pub async fn is_cached(&mut self, hash: &str) -> miette::Result<Option<HydrateFrom>> {
        let cache_engine = &self.app.cache_engine;

        debug!(
            task_target = self.task.target.as_str(),
            hash, "Checking if task has been cached using hash"
        );

        // If a lifetime has been configured, we need to check the last run and the archive
        // for staleness, and return a cache miss/skip
        let cache_lifetime = match &self.task.options.cache_lifetime {
            Some(lifetime) => Some(self.app.cache_engine.parse_lifetime(lifetime)?),
            None => None,
        };

        let is_cache_stale = || {
            if let Some(duration) = cache_lifetime {
                if is_stale(self.cache.data.last_run_time, duration) {
                    debug!(
                        task_target = self.task.target.as_str(),
                        hash,
                        "Cache skip, a lifetime has been configured and the last run is stale, continuing run"
                    );

                    return true;
                }
            }

            false
        };

        // If hash is the same as the previous build, we can simply abort!
        // However, ensure the outputs also exist, otherwise we should hydrate
        if self.cache.data.exit_code == 0
            && self.cache.data.hash == hash
            && self.archiver.has_outputs_been_created(true)?
        {
            if is_cache_stale() {
                return Ok(None);
            }

            debug!(
                task_target = self.task.target.as_str(),
                hash, "Hash matches previous run, reusing existing outputs"
            );

            return Ok(Some(HydrateFrom::PreviousOutput));
        }

        if !cache_engine.is_readable() {
            debug!(
                task_target = self.task.target.as_str(),
                hash, "Cache is not readable, continuing run"
            );

            return Ok(None);
        }

        // Set this *after* we checked the previous outputs above
        self.cache.data.hash = hash.to_owned();

        // If the previous run was a failure, avoid hydrating
        if self.cache.data.exit_code > 0 {
            debug!(
                task_target = self.task.target.as_str(),
                hash, "Previous run failed, avoiding hydration"
            );

            return Ok(None);
        }

        // Check if last run is stale
        if is_cache_stale() {
            return Ok(None);
        }

        // Check to see if a build with the provided hash has been cached locally.
        // We only check for the archive, as the manifest is purely for local debugging!
        let archive_file = cache_engine.hash.get_archive_path(hash);

        if archive_file.exists() {
            // Also check if the archive itself is stale
            if let Some(duration) = cache_lifetime {
                if fs::is_stale(&archive_file, false, duration, SystemTime::now())?.is_some() {
                    debug!(
                        task_target = self.task.target.as_str(),
                        hash,
                        archive_file = ?archive_file,
                        "Cache skip in local cache, a lifetime has been configured and the archive is stale, continuing run"
                    );

                    return Ok(None);
                }
            }

            debug!(
                task_target = self.task.target.as_str(),
                hash,
                archive_file = ?archive_file,
                "Cache hit in local cache, will reuse existing archive"
            );

            return Ok(Some(HydrateFrom::LocalCache));
        }

        // Check if the outputs have been cached in the remote service
        if let (Some(state), Some(remote)) = (&mut self.remote_state, RemoteService::session()) {
            if let Some(result) = remote.is_action_cached(state).await? {
                debug!(
                    task_target = self.task.target.as_str(),
                    hash, "Cache hit in remote service, will attempt to download output blobs"
                );

                state.set_action_result(result);

                return Ok(Some(HydrateFrom::RemoteCache));
            }
        }

        debug!(
            task_target = self.task.target.as_str(),
            hash, "Cache miss, continuing run"
        );

        Ok(None)
    }

    pub fn is_cache_enabled(&self) -> bool {
        // If the VCS root does not exist (like in a Docker container),
        // we should avoid failing and simply disable caching
        self.task.options.cache && self.app.vcs.is_enabled()
    }

    #[instrument(skip_all)]
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
                    task_target = self.task.target.as_str(),
                    dependency_target = dep.target.as_str(),
                    "Task dependency has failed or has been skipped, skipping this task",
                );

                return Ok(false);
            } else {
                return Err(TaskRunnerError::MissingDependencyHash {
                    dep_target: dep.target.clone(),
                    target: self.task.target.clone(),
                }
                .into());
            }
        }

        Ok(true)
    }

    #[instrument(skip_all)]
    pub async fn generate_hash(
        &mut self,
        context: &ActionContext,
        node: &ActionNode,
    ) -> miette::Result<String> {
        debug!(
            task_target = self.task.target.as_str(),
            "Generating a unique hash for this task"
        );

        let hash_engine = &self.app.cache_engine.hash;
        let mut hasher = hash_engine.create_hasher(node.label());
        let mut operation = Operation::hash_generation();

        // Hash common fields
        let mut task_hasher = TaskHasher::new(
            self.project,
            self.task,
            &self.app.vcs,
            &self.app.workspace_root,
            &self.app.workspace_config.hasher,
        );

        if self.task.script.is_none() && context.should_inherit_args(&self.task.target) {
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

        // Hash toolchain fields
        self.platform_manager
            .get_by_toolchains(&self.task.toolchains)?
            .hash_run_target(
                self.project,
                node.get_runtime(),
                &mut hasher,
                &self.app.workspace_config.hasher,
            )
            .await?;

        for content in self
            .app
            .toolchain_registry
            .hash_task_contents_many(
                self.project.get_enabled_toolchains(),
                |registry, toolchain| HashTaskContentsInput {
                    context: registry.create_context(),
                    project: self.project.to_fragment(),
                    task: self.task.to_fragment(),
                    toolchain_config: registry.create_merged_config(
                        &toolchain.id,
                        &self.app.toolchain_config,
                        &self.project.config,
                    ),
                },
            )
            .await?
        {
            hasher.hash_content(content)?;
        }

        // Generate the hash and persist values
        let hash = hash_engine.save_manifest(&mut hasher)?;

        operation.meta.set_hash(&hash);
        operation.finish(ActionStatus::Passed);

        self.operations.push(operation);
        self.report_item.hash = Some(hash.clone());

        if RemoteService::is_enabled() {
            let bytes = hasher.into_bytes();
            let mut state = ActionState::new(
                Digest {
                    hash: hash.clone(),
                    size_bytes: bytes.len() as i64,
                },
                self.task,
            );
            state.bytes = bytes;

            self.remote_state = Some(state);
        }

        debug!(
            task_target = self.task.target.as_str(),
            hash = &hash,
            "Generated a unique hash"
        );

        Ok(hash)
    }

    #[instrument(skip(self, context, node))]
    pub async fn execute(
        &mut self,
        context: &ActionContext,
        node: &ActionNode,
    ) -> miette::Result<()> {
        // If the task is a no-operation, we should exit early
        if self.task.is_no_op() {
            self.skip_no_op()?;

            return Ok(());
        }

        debug!(
            task_target = self.task.target.as_str(),
            "Building and executing the task command"
        );

        // Build the command from the current task
        let mut builder = CommandBuilder::new(self.app, self.project, self.task, node);
        builder.set_platform_manager(self.platform_manager);

        let command = builder.build(context).await?;

        // Execute the command and gather all attempts made
        let executor = CommandExecutor::new(self.app, self.project, self.task, node, command);

        let result = if let Some(mutex_name) = &self.task.options.mutex {
            let mut operation = Operation::mutex_acquisition();

            trace!(
                task_target = self.task.target.as_str(),
                mutex = mutex_name,
                "Waiting to acquire task mutex lock"
            );

            let mutex = context.get_or_create_mutex(mutex_name);
            let _guard = mutex.lock().await;

            trace!(
                task_target = self.task.target.as_str(),
                mutex = mutex_name,
                "Acquired task mutex lock"
            );

            operation.finish(ActionStatus::Passed);

            self.operations.push(operation);

            // This execution is required within this block so that the
            // guard above isn't immediately dropped!
            executor.execute(context, &mut self.report_item).await?
        } else {
            executor.execute(context, &mut self.report_item).await?
        };

        // Persist the state locally and for the remote service
        if let Some(last_attempt) = result.attempts.get_last_execution() {
            self.persist_state(last_attempt)?;

            if let Some(state) = &mut self.remote_state {
                state.create_action_result_from_operation(last_attempt)?;
            }
        }

        // Extract the attempts from the result
        self.operations.merge(result.attempts);

        // Update the action state based on the result
        self.target_state = Some(result.run_state);

        // If the execution as a whole failed, return the error.
        // We do this here instead of in `execute` so that we can
        // capture the attempts and report them.
        if let Some(result_error) = result.error {
            return Err(result_error);
        }

        // If our last task execution was a failure, return a hard error
        if let Some(last_attempt) = self.operations.get_last_execution() {
            if last_attempt.has_failed() {
                return Err(TaskRunnerError::RunFailed {
                    target: self.task.target.clone(),
                    error: Box::new(ProcessError::ExitNonZero {
                        bin: self.task.command.clone(),
                        status: last_attempt.get_exec_output_status(),
                    }),
                }
                .into());
            }
        }

        Ok(())
    }

    #[instrument(skip(self))]
    pub fn skip(&mut self) -> miette::Result<()> {
        debug!(task_target = self.task.target.as_str(), "Skipping task");

        self.operations.push(Operation::new_finished(
            OperationMeta::TaskExecution(Default::default()),
            ActionStatus::Skipped,
        ));

        self.target_state = Some(TargetState::Skipped);

        Ok(())
    }

    #[instrument(skip(self))]
    pub fn skip_no_op(&mut self) -> miette::Result<()> {
        debug!(
            task_target = self.task.target.as_str(),
            "Skipping task as its a no-operation"
        );

        self.operations.push(Operation::new_finished(
            OperationMeta::NoOperation,
            ActionStatus::Passed,
        ));

        self.target_state = Some(TargetState::from_hash(self.report_item.hash.as_deref()));

        Ok(())
    }

    #[instrument(skip(self))]
    pub async fn archive(&mut self, hash: &str) -> miette::Result<bool> {
        let mut operation = Operation::archive_creation();

        debug!(
            task_target = self.task.target.as_str(),
            "Running cache archiving operation"
        );

        let archived = self
            .archiver
            .archive(hash, self.remote_state.as_mut())
            .await?
            .is_some();

        if archived {
            debug!(
                task_target = self.task.target.as_str(),
                "Ran cache archiving operation"
            );

            operation.finish(ActionStatus::Passed);
        } else {
            debug!(
                task_target = self.task.target.as_str(),
                "Nothing to archive"
            );

            operation.finish(ActionStatus::Skipped);
        }

        self.operations.push(operation);

        Ok(archived)
    }

    #[instrument(skip(self))]
    pub async fn hydrate(&mut self, hash: &str) -> miette::Result<bool> {
        let mut operation = Operation::output_hydration();

        // Not cached
        let Some(from) = self.is_cached(hash).await? else {
            debug!(
                task_target = self.task.target.as_str(),
                "Nothing to hydrate"
            );

            operation.finish(ActionStatus::Skipped);

            self.operations.push(operation);

            return Ok(false);
        };

        // Did not hydrate
        debug!(
            task_target = self.task.target.as_str(),
            hydrate_from = ?from,
            "Running cache hydration operation"
        );

        if !self
            .hydrater
            .hydrate(from, hash, self.remote_state.as_mut())
            .await?
        {
            debug!(task_target = self.task.target.as_str(), "Did not hydrate");

            operation.finish(ActionStatus::Invalid);

            self.operations.push(operation);

            return Ok(false);
        }

        // Did hydrate
        debug!(
            task_target = self.task.target.as_str(),
            "Ran cache hydration operation"
        );

        // Fill in these values since the command executor does not run!
        if let Some(output) = operation.get_exec_output_mut() {
            output.command = Some(self.task.get_command_line());

            // If we received an action result from the remote cache,
            // extract the logs from it
            if let Some(result) = self
                .remote_state
                .as_ref()
                .and_then(|state| state.action_result.as_ref())
            {
                output.exit_code = Some(result.exit_code);

                if !result.stderr_raw.is_empty() {
                    output.set_stderr(String::from_utf8_lossy(&result.stderr_raw).into());
                }

                if !result.stdout_raw.is_empty() {
                    output.set_stdout(String::from_utf8_lossy(&result.stdout_raw).into());
                }
            }
            // If not from the remote cache, we need to read the locally
            // cached stdout/stderr log files
            else {
                output.exit_code = Some(self.cache.data.exit_code);

                let state_dir = self
                    .app
                    .cache_engine
                    .state
                    .get_target_dir(&self.task.target);
                let err_path = state_dir.join("stderr.log");
                let out_path = state_dir.join("stdout.log");

                if err_path.exists() {
                    output.set_stderr(fs::read_file(err_path)?);
                }

                if out_path.exists() {
                    output.set_stdout(fs::read_file(out_path)?);
                }
            }
        }

        // Then finalize the operation and target state
        operation.finish(match from {
            HydrateFrom::RemoteCache => ActionStatus::CachedFromRemote,
            _ => ActionStatus::Cached,
        });

        self.persist_state(&operation)?;

        self.operations.push(operation);
        self.target_state = Some(TargetState::Passed(hash.to_owned()));

        Ok(true)
    }

    // If a task fails *before* the command is actually executed, say during the command
    // build process, or the toolchain plugin layer, that error is not bubbled up as a
    // failure, and the last operation is used instead (which is typically skipped).
    // To handle this weird scenario, we inject a failed task execution at the end.
    fn inject_failed_task_execution(
        &mut self,
        report: Option<&miette::Report>,
    ) -> miette::Result<()> {
        let has_exec = self
            .operations
            .iter()
            .any(|operation| operation.meta.is_task_execution());

        if has_exec {
            return Ok(());
        }

        let mut operation = Operation::task_execution(&self.task.command);

        if let Some(output) = operation.get_exec_output_mut() {
            output.exit_code = Some(-1);
        }

        operation.finish(ActionStatus::Aborted);

        self.app.console.on_task_finished(
            &self.task.target,
            &operation,
            &self.report_item,
            report,
        )?;

        self.operations.push(operation);

        Ok(())
    }

    fn persist_state(&mut self, operation: &Operation) -> miette::Result<()> {
        let state_dir = self
            .app
            .cache_engine
            .state
            .get_target_dir(&self.task.target);
        let err_path = state_dir.join("stderr.log");
        let out_path = state_dir.join("stdout.log");

        if let Some(output) = operation.get_exec_output() {
            self.cache.data.exit_code = output.get_exit_code();

            fs::write_file(
                err_path,
                output
                    .stderr
                    .as_ref()
                    .map(|log| log.as_bytes())
                    .unwrap_or_default(),
            )?;

            fs::write_file(
                out_path,
                output
                    .stdout
                    .as_ref()
                    .map(|log| log.as_bytes())
                    .unwrap_or_default(),
            )?;
        } else {
            // Ensure logs from a previous run are removed
            fs::remove_file(err_path)?;
            fs::remove_file(out_path)?;
        }

        Ok(())
    }
}
