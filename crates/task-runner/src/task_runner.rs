use crate::command_builder::CommandBuilder;
use crate::command_executor::CommandExecutor;
use crate::labels::action_status_label;
#[cfg(feature = "otel")]
use crate::metrics::task_runner_metrics;
use crate::output_archiver::OutputArchiver;
use crate::output_hydrater::{HydrateFrom, OutputHydrater};
use crate::run_state::*;
use crate::task_runner_error::TaskRunnerError;
use moon_action::{ActionNode, ActionStatus, Operation, OperationList, OperationMeta};
use moon_action_context::{ActionContext, TargetState};
use moon_app_context::AppContext;
use moon_cache::CacheItem;
use moon_common::is_ci_env;
use moon_console::TaskReportItem;
use moon_process::ProcessError;
use moon_project::Project;
use moon_remote::{ActionState, Digest, RemoteService};
use moon_task::Task;
use moon_task_hasher::*;
use moon_time::{is_stale, now_millis};
use starbase_utils::fs;
use std::sync::Arc;
#[cfg(feature = "otel")]
use std::time::Instant;
use tracing::{Span, debug, instrument, trace};

#[derive(Debug)]
pub struct TaskRunResult {
    pub hash: Option<String>,
    pub error: Option<miette::Report>,
    pub operations: OperationList,
}

pub struct TaskRunner<'task> {
    app_context: &'task Arc<AppContext>,
    project: &'task Arc<Project>,
    pub task: &'task Arc<Task>,

    archiver: OutputArchiver<'task>,
    hydrater: OutputHydrater<'task>,

    // Public for testing
    pub cache: CacheItem<TaskRunCacheState>,
    pub operations: OperationList,
    pub remote_state: Option<ActionState<'task>>,
    pub report_item: TaskReportItem,
    pub trace_cache_source: Option<&'static str>,
    pub target_state: Option<TargetState>,
}

impl<'task> TaskRunner<'task> {
    pub fn new(
        app_context: &'task Arc<AppContext>,
        project: &'task Arc<Project>,
        task: &'task Arc<Task>,
    ) -> miette::Result<Self> {
        debug!(
            task_target = task.target.as_str(),
            "Creating a task runner for target"
        );

        let mut cache = app_context
            .cache_engine
            .state
            .load_target_state::<TaskRunCacheState>(&task.target)?;

        if cache.data.target.is_empty() {
            cache.data.target = task.target.to_string();
        }

        Ok(Self {
            cache,
            archiver: OutputArchiver { app_context, task },
            hydrater: OutputHydrater { app_context, task },
            project,
            remote_state: None,
            report_item: TaskReportItem {
                output_style: task.options.output_style,
                ..Default::default()
            },
            trace_cache_source: None,
            target_state: None,
            task,
            app_context,
            operations: OperationList::default(),
        })
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

        // Always generate a hash
        let hash = self.generate_hash(context, node).await?;

        if self.is_cache_enabled() {
            debug!(
                task_target = self.task.target.as_str(),
                "Caching is enabled for task, will attempt to hydrate and archive outputs"
            );

            // Exit early if this build has already been cached/hashed
            if self.hydrate(&hash).await? {
                return Ok(Some(hash));
            }

            // Otherwise build and execute the command as a child process
            self.execute(context, node).await?;

            // If we created outputs, archive them into the cache
            self.archive(&hash).await?;
        } else {
            debug!(
                task_target = self.task.target.as_str(),
                "Caching is disabled for task, will attempt to run the command as normal"
            );

            // Build and execute without managing cache
            self.execute(context, node).await?;
        }

        Ok(Some(hash))
    }

    fn record_cache_lookup(&mut self, span: &Span, hit: bool, source: &'static str) {
        self.trace_cache_source = Some(source);
        span.record("cache_hit", hit);
        span.record("cache_source", source);
    }

    fn record_status(span: &Span, status: ActionStatus) {
        span.record("status", action_status_label(status));
    }

    fn get_trace_cache_source(&self) -> &'static str {
        self.trace_cache_source.unwrap_or_else(|| {
            if self.is_cache_enabled() {
                "miss"
            } else {
                "disabled"
            }
        })
    }

    #[instrument(
        name = "task_run",
        skip(self, context, node),
        fields(
            project_id = %self.project.id,
            task_target = %self.task.target,
            task_id = %self.task.id,
            task_type = %self.task.type_of,
            interactive = node.is_interactive() || self.task.is_interactive(),
            persistent = node.is_persistent() || self.task.is_persistent(),
            retry_total = self.task.options.retry_count + 1,
            cache_enabled = self.is_cache_enabled(),
            ci = is_ci_env(),
            status = tracing::field::Empty,
            cache_hit = tracing::field::Empty,
            cache_source = tracing::field::Empty,
            exit_code = tracing::field::Empty,
            flaky = tracing::field::Empty,
        )
    )]
    pub async fn run(
        &mut self,
        context: &ActionContext,
        node: &ActionNode,
    ) -> miette::Result<TaskRunResult> {
        #[cfg(feature = "otel")]
        let run_started = Instant::now();
        let span = Span::current();
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

                self.app_context.console.on_task_completed(
                    &self.task.target,
                    &self.operations,
                    &self.report_item,
                    None,
                )?;

                let status = self.operations.get_final_status();
                Self::record_status(&span, status);
                span.record("cache_hit", self.operations.iter().any(|op| op.is_cached()));
                let cache_source = self.get_trace_cache_source();
                span.record("cache_source", cache_source);
                span.record("flaky", self.operations.is_flaky());
                #[cfg(feature = "otel")]
                task_runner_metrics().record_task_run(
                    self.task,
                    status,
                    cache_source,
                    node.is_interactive() || self.task.is_interactive(),
                    node.is_persistent() || self.task.is_persistent(),
                    run_started.elapsed(),
                );

                if let Some(exit_code) = self
                    .operations
                    .get_last_process()
                    .and_then(|operation| operation.get_exec_output())
                    .and_then(|output| output.exit_code)
                {
                    span.record("exit_code", exit_code);
                }

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

                self.app_context.console.on_task_completed(
                    &self.task.target,
                    &self.operations,
                    &self.report_item,
                    Some(&error),
                )?;

                let status = self.operations.get_final_status();
                Self::record_status(&span, status);
                span.record("cache_hit", self.operations.iter().any(|op| op.is_cached()));
                let cache_source = self.get_trace_cache_source();
                span.record("cache_source", cache_source);
                span.record("flaky", self.operations.is_flaky());
                #[cfg(feature = "otel")]
                task_runner_metrics().record_task_run(
                    self.task,
                    status,
                    cache_source,
                    node.is_interactive() || self.task.is_interactive(),
                    node.is_persistent() || self.task.is_persistent(),
                    run_started.elapsed(),
                );

                if let Some(exit_code) = self
                    .operations
                    .get_last_process()
                    .and_then(|operation| operation.get_exec_output())
                    .and_then(|output| output.exit_code)
                {
                    span.record("exit_code", exit_code);
                }

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

    #[instrument(
        name = "task_cache_lookup",
        skip(self, hash),
        fields(
            project_id = %self.project.id,
            task_target = %self.task.target,
            task_id = %self.task.id,
            cache_hit = tracing::field::Empty,
            cache_source = tracing::field::Empty,
        )
    )]
    pub async fn is_cached(&mut self, hash: &str) -> miette::Result<Option<HydrateFrom>> {
        #[cfg(feature = "otel")]
        let lookup_started = Instant::now();
        let span = Span::current();
        let cache_engine = &self.app_context.cache_engine;

        debug!(
            task_target = self.task.target.as_str(),
            hash, "Checking if task has been cached using hash"
        );

        // If a lifetime has been configured, we need to check the last run and the archive
        // for staleness, and return a cache miss/skip
        let cache_lifetime = match &self.task.options.cache_lifetime {
            Some(lifetime) => Some(self.app_context.cache_engine.parse_lifetime(lifetime)?),
            None => None,
        };

        let is_cache_stale = || {
            if let Some(duration) = cache_lifetime
                && self.cache.data.last_run_time > 0
                && is_stale(self.cache.data.last_run_time, duration)
            {
                debug!(
                    task_target = self.task.target.as_str(),
                    hash,
                    "Cache skip, a lifetime has been configured and the last run is stale, continuing run"
                );

                return true;
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

            self.record_cache_lookup(&span, true, "previous-output");
            #[cfg(feature = "otel")]
            task_runner_metrics().record_cache_lookup(
                self.task,
                "previous-output",
                true,
                lookup_started.elapsed(),
            );
            return Ok(Some(HydrateFrom::PreviousOutput));
        }

        if !cache_engine.is_readable() {
            debug!(
                task_target = self.task.target.as_str(),
                hash, "Cache is not readable, continuing run"
            );

            self.record_cache_lookup(&span, false, "unreadable");
            #[cfg(feature = "otel")]
            task_runner_metrics().record_cache_lookup(
                self.task,
                "unreadable",
                false,
                lookup_started.elapsed(),
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

            self.record_cache_lookup(&span, false, "previous-failure");
            #[cfg(feature = "otel")]
            task_runner_metrics().record_cache_lookup(
                self.task,
                "previous-failure",
                false,
                lookup_started.elapsed(),
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

        if archive_file.exists() && self.task.options.cache.is_local_enabled() {
            // Also check if the archive itself is stale
            if let Some(duration) = cache_lifetime
                && fs::is_stale(&archive_file, false, duration)?
            {
                debug!(
                    task_target = self.task.target.as_str(),
                    hash,
                    archive_file = ?archive_file,
                    "Cache skip in local cache, a lifetime has been configured and the archive is stale, continuing run"
                );

                return Ok(None);
            }

            debug!(
                task_target = self.task.target.as_str(),
                hash,
                archive_file = ?archive_file,
                "Cache hit in local cache, will reuse existing archive"
            );

            self.record_cache_lookup(&span, true, "local-cache");
            #[cfg(feature = "otel")]
            task_runner_metrics().record_cache_lookup(
                self.task,
                "local-cache",
                true,
                lookup_started.elapsed(),
            );
            return Ok(Some(HydrateFrom::LocalCache));
        }

        // Check if the outputs have been cached in the remote service
        if self.task.options.cache.is_remote_enabled()
            && let (Some(state), Some(remote)) = (&mut self.remote_state, RemoteService::session())
            // Don't bubble up errors from the remote cache check, just treat them as cache misses
            && let Ok(Some(result)) = remote.is_action_cached(state).await
        {
            debug!(
                task_target = self.task.target.as_str(),
                hash, "Cache hit in remote service, will attempt to download output blobs"
            );

            state.set_action_result(result);

            self.record_cache_lookup(&span, true, "remote-cache");
            #[cfg(feature = "otel")]
            task_runner_metrics().record_cache_lookup(
                self.task,
                "remote-cache",
                true,
                lookup_started.elapsed(),
            );
            return Ok(Some(HydrateFrom::RemoteCache));
        }

        debug!(
            task_target = self.task.target.as_str(),
            hash, "Cache miss, continuing run"
        );

        self.record_cache_lookup(&span, false, "miss");
        #[cfg(feature = "otel")]
        task_runner_metrics().record_cache_lookup(
            self.task,
            "miss",
            false,
            lookup_started.elapsed(),
        );
        Ok(None)
    }

    pub fn is_cache_enabled(&self) -> bool {
        // If the VCS root does not exist (like in a Docker container),
        // we should avoid failing and simply disable caching
        self.task.options.cache.is_enabled() && self.app_context.vcs.is_enabled()
    }

    #[instrument(skip_all)]
    pub fn is_dependencies_complete(&self, context: &ActionContext) -> miette::Result<bool> {
        if self.task.deps.is_empty() {
            return Ok(true);
        }

        for dep in &self.task.deps {
            if let Some(dep_state) = context.target_states.get_sync(&dep.target) {
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

    #[instrument(
        name = "task_hash_generation",
        skip_all,
        fields(
            project_id = %self.project.id,
            task_target = %self.task.target,
            task_id = %self.task.id,
        )
    )]
    pub async fn generate_hash(
        &mut self,
        context: &ActionContext,
        node: &ActionNode,
    ) -> miette::Result<String> {
        #[cfg(feature = "otel")]
        let hash_started = Instant::now();
        debug!(
            task_target = self.task.target.as_str(),
            "Generating a unique hash for this task"
        );

        let hash_engine = &self.app_context.cache_engine.hash;
        let mut hasher = hash_engine.create_hasher(node.label());
        let mut operation = Operation::hash_generation();

        // Hash common fields
        hash_common_task_contents(
            self.app_context,
            context,
            self.project,
            self.task,
            node,
            &mut hasher,
        )
        .await?;

        // Hash toolchain fields
        hash_toolchain_task_contents(self.app_context, self.project, self.task, &mut hasher)
            .await?;

        // Generate the hash and persist values
        let hash = hash_engine.save_manifest(&mut hasher)?;

        operation.meta.set_hash(&hash);
        operation.finish(ActionStatus::Passed);

        self.operations.push(operation);
        self.report_item.hash = Some(hash.clone());

        // Store the hash digest for remote caching
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

        #[cfg(feature = "otel")]
        task_runner_metrics().record_hash_generation(self.task, hash_started.elapsed());

        Ok(hash)
    }

    #[instrument(
        name = "task_execution",
        skip(self, context, node),
        fields(
            project_id = %self.project.id,
            task_target = %self.task.target,
            task_id = %self.task.id,
            interactive = node.is_interactive() || self.task.is_interactive(),
            persistent = node.is_persistent() || self.task.is_persistent(),
            retry_total = self.task.options.retry_count + 1,
            status = tracing::field::Empty,
            exit_code = tracing::field::Empty,
        )
    )]
    pub async fn execute(
        &mut self,
        context: &ActionContext,
        node: &ActionNode,
    ) -> miette::Result<()> {
        #[cfg(feature = "otel")]
        let execution_started = Instant::now();
        let span = Span::current();
        if self.task.is_no_op() {
            self.skip_no_op()?;
            Self::record_status(&span, ActionStatus::Passed);
            #[cfg(feature = "otel")]
            task_runner_metrics().record_execution(
                self.task,
                ActionStatus::Passed,
                node.is_interactive() || self.task.is_interactive(),
                node.is_persistent() || self.task.is_persistent(),
                execution_started.elapsed(),
            );

            return Ok(());
        }

        debug!(
            task_target = self.task.target.as_str(),
            "Building and executing the task command"
        );

        // Build the command from the current task
        let command = CommandBuilder::new(self.app_context, self.project, self.task, node)
            .build(
                context,
                self.report_item.hash.as_deref().unwrap_or_default(),
            )
            .await?;

        // Execute the command and gather all attempts made
        let executor =
            CommandExecutor::new(self.app_context, self.project, self.task, node, command);

        let result = if let Some(mutex_name) = &self.task.options.mutex {
            let mut operation = Operation::mutex_acquisition();

            trace!(
                task_target = self.task.target.as_str(),
                mutex = mutex_name,
                "Waiting to acquire task mutex lock"
            );

            let mutex = context.get_or_create_mutex(mutex_name).await;
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

            Self::record_status(&span, last_attempt.status);

            if let Some(exit_code) = last_attempt
                .get_exec_output()
                .and_then(|output| output.exit_code)
            {
                span.record("exit_code", exit_code);
            }

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
            #[cfg(feature = "otel")]
            task_runner_metrics().record_execution(
                self.task,
                self.operations
                    .get_last_execution()
                    .map(|attempt| attempt.status)
                    .unwrap_or(ActionStatus::Failed),
                node.is_interactive() || self.task.is_interactive(),
                node.is_persistent() || self.task.is_persistent(),
                execution_started.elapsed(),
            );
            return Err(result_error);
        }

        // If our last task execution was a failure, return a hard error
        if let Some(last_attempt) = self.operations.get_last_execution()
            && last_attempt.has_failed()
        {
            #[cfg(feature = "otel")]
            task_runner_metrics().record_execution(
                self.task,
                last_attempt.status,
                node.is_interactive() || self.task.is_interactive(),
                node.is_persistent() || self.task.is_persistent(),
                execution_started.elapsed(),
            );
            return Err(TaskRunnerError::RunFailed {
                target: self.task.target.clone(),
                error: Box::new(ProcessError::ExitNonZero {
                    bin: self.task.command.value.clone(),
                    status: last_attempt.get_exec_output_status(),
                }),
            }
            .into());
        }

        #[cfg(feature = "otel")]
        {
            let execution_status = self
                .operations
                .get_last_execution()
                .map(|attempt| attempt.status)
                .unwrap_or(ActionStatus::Passed);

            task_runner_metrics().record_execution(
                self.task,
                execution_status,
                node.is_interactive() || self.task.is_interactive(),
                node.is_persistent() || self.task.is_persistent(),
                execution_started.elapsed(),
            );
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

    #[instrument(
        name = "task_output_archive",
        skip(self, hash),
        fields(
            project_id = %self.project.id,
            task_target = %self.task.target,
            task_id = %self.task.id,
            archived = tracing::field::Empty,
        )
    )]
    pub async fn archive(&mut self, hash: &str) -> miette::Result<bool> {
        #[cfg(feature = "otel")]
        let archive_started = Instant::now();
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
        Span::current().record("archived", archived);
        #[cfg(feature = "otel")]
        task_runner_metrics().record_archive(
            self.task,
            archived,
            self.operations
                .iter()
                .last()
                .map(|operation| operation.status)
                .unwrap_or(ActionStatus::Skipped),
            archive_started.elapsed(),
        );

        Ok(archived)
    }

    #[instrument(
        name = "task_output_hydration",
        skip(self, hash),
        fields(
            project_id = %self.project.id,
            task_target = %self.task.target,
            task_id = %self.task.id,
            hydrate_from = tracing::field::Empty,
            hydrated = tracing::field::Empty,
            status = tracing::field::Empty,
        )
    )]
    pub async fn hydrate(&mut self, hash: &str) -> miette::Result<bool> {
        #[cfg(feature = "otel")]
        let hydration_started = Instant::now();
        let span = Span::current();
        let mut operation = Operation::output_hydration();

        // Not cached
        let Some(from) = self.is_cached(hash).await? else {
            debug!(
                task_target = self.task.target.as_str(),
                "Nothing to hydrate"
            );

            operation.finish(ActionStatus::Skipped);

            self.operations.push(operation);
            Self::record_status(&span, ActionStatus::Skipped);
            span.record("hydrated", false);
            span.record("hydrate_from", "miss");
            #[cfg(feature = "otel")]
            task_runner_metrics().record_hydration(
                self.task,
                "miss",
                ActionStatus::Skipped,
                false,
                hydration_started.elapsed(),
            );

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
            Self::record_status(&span, ActionStatus::Invalid);
            span.record("hydrated", false);
            let hydrate_from = match from {
                HydrateFrom::LocalCache => "local-cache",
                HydrateFrom::PreviousOutput => "previous-output",
                HydrateFrom::RemoteCache => "remote-cache",
            };
            span.record("hydrate_from", hydrate_from);
            #[cfg(feature = "otel")]
            task_runner_metrics().record_hydration(
                self.task,
                hydrate_from,
                ActionStatus::Invalid,
                false,
                hydration_started.elapsed(),
            );

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
                    .app_context
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

        let status = operation.status;

        self.operations.push(operation);
        self.target_state = Some(TargetState::Passed(hash.to_owned()));
        span.record("hydrated", true);
        Self::record_status(&span, status);
        let hydrate_from = match from {
            HydrateFrom::LocalCache => "local-cache",
            HydrateFrom::PreviousOutput => "previous-output",
            HydrateFrom::RemoteCache => "remote-cache",
        };
        span.record("hydrate_from", hydrate_from);
        #[cfg(feature = "otel")]
        task_runner_metrics().record_hydration(
            self.task,
            hydrate_from,
            status,
            true,
            hydration_started.elapsed(),
        );

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

        self.app_context.console.on_task_finished(
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
            .app_context
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
