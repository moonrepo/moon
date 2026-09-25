use crate::abort_state::{AbortState, DrainedGuard};
use crate::event_emitter::{Event, EventEmitter};
use crate::job::Job;
use crate::job_context::{CompletedJob, JobContext};
use crate::job_dispatcher::JobDispatcher;
use crate::subscribers::cleanup_subscriber::CleanupSubscriber;
use crate::subscribers::console_subscriber::ConsoleSubscriber;
use crate::subscribers::metrics_subscriber::MetricsSubscriber;
use crate::subscribers::notifications_subscriber::NotificationsSubscriber;
use crate::subscribers::reports_subscriber::ReportsSubscriber;
// use crate::subscribers::telemetry_subscriber::TelemetrySubscriber;
use crate::subscribers::webhooks_subscriber::WebhooksSubscriber;
use miette::IntoDiagnostic;
use moon_action::{Action, ActionNode, ActionPipelineStatus, ActionStatus};
use moon_action_context::{ActionContext, TargetState};
use moon_action_graph::ActionGraph;
use moon_app_context::AppContext;
use moon_common::{color, is_remote, is_test_env};
use moon_console::Level;
use moon_daemon_client::DaemonClient;
use moon_process::{ProcessRegistry, SignalType};
use moon_workspace_graph::WorkspaceGraph;
use rustc_hash::FxHashMap;
use std::mem;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::broadcast::error::RecvError;
use tokio::sync::{RwLock, Semaphore, mpsc};
use tokio::task::{JoinHandle, JoinSet};
use tokio_util::sync::CancellationToken;
use tracing::{debug, instrument, warn};

/// Extra time to wait after the process registry's kill threshold, before
/// starting new processes once running processes have been terminated.
const TERMINATION_GRACE_PERIOD: Duration = Duration::from_millis(250);

pub struct ActionPipeline {
    pub bail: bool,
    pub concurrency: usize,
    pub quiet: bool,
    pub report_name: String,
    pub summary: Option<Level>,

    // State
    actions: Vec<Action>,
    duration: Option<Duration>,
    status: ActionPipelineStatus,

    // Data
    app_context: Arc<AppContext>,
    action_context: Arc<ActionContext>,
    daemon_client: Option<DaemonClient>,
    emitter: Arc<EventEmitter>,
    workspace_graph: Arc<WorkspaceGraph>,
}

/// Handles of all jobs currently dispatched in the pipeline.
#[derive(Default)]
struct JobHandles {
    /// Handles for non-persistent jobs, which eventually complete.
    standard: JoinSet<()>,

    /// Handles for persistent jobs, which never complete on their own,
    /// and as such, must never be awaited alongside standard jobs.
    persistent: JoinSet<()>,
}

impl JobHandles {
    fn is_empty(&self) -> bool {
        self.standard.is_empty() && self.persistent.is_empty()
    }

    async fn shutdown(&mut self) {
        self.standard.shutdown().await;
        self.persistent.shutdown().await;
    }
}

impl ActionPipeline {
    pub fn new(
        app_context: Arc<AppContext>,
        workspace_graph: Arc<WorkspaceGraph>,
        daemon_client: Option<DaemonClient>,
    ) -> Self {
        debug!("Creating pipeline to run actions");

        Self {
            action_context: Arc::new(ActionContext::default()),
            actions: vec![],
            app_context,
            bail: false,
            concurrency: num_cpus::get(),
            daemon_client,
            duration: None,
            emitter: Arc::new(EventEmitter::default()),
            quiet: false,
            report_name: "runReport.json".into(),
            status: ActionPipelineStatus::Pending,
            summary: None,
            workspace_graph,
        }
    }

    pub async fn run(self, action_graph: ActionGraph) -> miette::Result<Vec<Action>> {
        self.run_with_context(action_graph, ActionContext::default())
            .await
    }

    #[instrument(name = "run_pipeline", skip_all)]
    pub async fn run_with_context(
        mut self,
        action_graph: ActionGraph,
        action_context: ActionContext,
    ) -> miette::Result<Vec<Action>> {
        self.action_context = Arc::new(action_context);
        self.setup_subscribers().await;

        self.emitter
            .emit(Event::PipelineStarted {
                actions_count: action_graph.get_node_count(),
                action_nodes: action_graph.get_nodes(),
                context: &self.action_context,
            })
            .await?;

        // Run the pipeline based on the graph
        let result = self.internal_run(action_graph).await;
        let actions = mem::take(&mut self.actions);

        // Handle the result of the pipeline
        match result {
            Ok(_) => {
                self.emitter
                    .emit(Event::PipelineCompleted {
                        actions: &actions,
                        context: &self.action_context,
                        duration: self.duration,
                        error: None,
                        error_report: None,
                        status: &self.status,
                    })
                    .await?;

                Ok(actions)
            }
            Err(error) => {
                self.emitter
                    .emit(Event::PipelineCompleted {
                        actions: &actions,
                        context: &self.action_context,
                        duration: self.duration,
                        error: Some(error.to_string()),
                        error_report: Some(&error),
                        status: &self.status,
                    })
                    .await?;

                Err(error)
            }
        }
    }

    pub async fn internal_run(&mut self, action_graph: ActionGraph) -> miette::Result<()> {
        let total_actions = action_graph.get_node_count();
        let start = Instant::now();

        if total_actions == 0 {
            debug!(total_actions, "No actions available, not running pipeline");

            return Ok(());
        }

        debug!(
            total_actions,
            concurrency = self.concurrency,
            "Starting pipeline"
        );

        // This aggregates results from jobs
        let (sender, mut receiver) = mpsc::channel::<Action>(total_actions.max(1));

        // This aggregates completed jobs for the dispatcher
        let (completed_sender, completed_receiver) = mpsc::unbounded_channel::<CompletedJob>();

        // Create job context
        let abort_state = Arc::new(AbortState::default());
        let abort_token = CancellationToken::new();
        let cancel_token = CancellationToken::new();

        let job_context = JobContext {
            abort_token: abort_token.clone(),
            abort_state: Arc::clone(&abort_state),
            bail: self.bail,
            cancel_token: cancel_token.clone(),
            completed_queue: completed_sender,
            daemon_client: self.daemon_client.clone(),
            emitter: Arc::clone(&self.emitter),
            result_sender: sender,
            semaphore: Arc::new(Semaphore::new(self.concurrency)),
            running_jobs: Arc::new(RwLock::new(FxHashMap::default())),
            workspace_graph: self.workspace_graph.clone(),
        };

        // Cleanup jobs may need to run after the pipeline was aborted
        let has_cleanups = !action_graph.get_cleanup_indices().is_empty();

        // Monitor signals and ctrl+c
        let signal_handle = self.monitor_signals(cancel_token.clone(), Arc::clone(&abort_state));

        // Dispatch jobs from the graph to run actions
        let queue_handle =
            self.dispatch_jobs(action_graph, job_context.clone(), completed_receiver)?;

        // Wait and receive all results coming through
        debug!("Waiting for jobs to return results");

        let process_registry = ProcessRegistry::instance();
        let mut actions = vec![];
        let mut aborted = false;
        let mut draining = false;
        let mut error = None;

        loop {
            let received = if draining {
                tokio::select! {
                    // Prefer results, so that none are left behind
                    biased;

                    action = receiver.recv() => action,

                    // All cleanup jobs have ran and sent their results,
                    // so receive the ones that remain, and stop
                    _ = abort_state.wait_until_drained() => {
                        receiver.close();
                        receiver.recv().await
                    }

                    // Or a signal stops waiting on cleanup jobs
                    _ = cancel_token.cancelled() => {
                        receiver.close();
                        receiver.recv().await
                    }
                }
            } else {
                receiver.recv().await
            };

            let Some(mut action) = received else {
                break;
            };

            // Only abort once, as the process registry only shuts down running
            // processes once, and every termination is broadcast like a signal
            if !aborted && job_context.should_abort(&action) {
                aborted = true;

                abort_state.mark_terminated();
                process_registry.terminate_running();
                abort_token.cancel();

                // The registry force kills its processes after the threshold, so
                // new processes must not be started until then, and those that
                // are still running after another threshold are no longer waited
                // on (a threshold of 0 lets processes run to completion instead)
                let threshold = Duration::from_millis(process_registry.threshold as u64);
                let now = Instant::now();

                abort_state.handle(
                    // Cleanup jobs run after a failure, but not after a signal
                    has_cleanups && !cancel_token.is_cancelled(),
                    (!threshold.is_zero()).then(|| now + threshold + TERMINATION_GRACE_PERIOD),
                    (!threshold.is_zero()).then(|| now + threshold * 2 + TERMINATION_GRACE_PERIOD),
                );
            }

            // Only bubble up an error on a hard failure, otherwise we can
            // continue to run and collect other actions. Keep the first
            // error, as it's the closest to the root cause — later failures
            // are typically fallout from running in the already-broken state.
            // Jobs that were terminated because a sibling failed are not the
            // cause, but carry the error of their termination, and may arrive
            // before the failing sibling
            if action.should_abort()
                && action.has_error()
                && error.is_none()
                && !abort_state.is_casualty(action.node_index)
            {
                error = Some(action.get_error());
            }

            actions.push(action);

            if aborted {
                if self.status != ActionPipelineStatus::Aborted {
                    debug!("Aborting pipeline (because something failed)");

                    self.status = ActionPipelineStatus::Aborted;
                }

                // Continue receiving results until the cleanup jobs have ran
                if abort_state.should_drain() {
                    draining = true;
                } else {
                    receiver.close();
                }
            } else if cancel_token.is_cancelled() {
                debug!("Cancelling pipeline (because a signal)");

                self.status = ActionPipelineStatus::Interrupted;
                receiver.close();
            } else if actions.len() == total_actions {
                debug!("Finished pipeline, received all results");

                self.status = ActionPipelineStatus::Completed;
                break;
            }
        }

        drop(receiver);

        // The dispatcher may be waiting on the abort to be handled
        abort_state.handle(false, None, None);

        // Capture and handle any signals
        if cancel_token.is_cancelled() && self.status == ActionPipelineStatus::Pending {
            self.status = match signal_handle.await.into_diagnostic()? {
                SignalType::Interrupt => ActionPipelineStatus::Interrupted,
                SignalType::Terminate => ActionPipelineStatus::Terminated,
                _ => ActionPipelineStatus::Aborted,
            };
        } else {
            signal_handle.abort();
        }

        // Wait for running child processes to exit
        process_registry.wait_for_running_to_shutdown().await;

        // Abort any running actions in progress
        if !matches!(self.status, ActionPipelineStatus::Completed) {
            if self.bail {
                queue_handle.abort();
            } else {
                let mut job_handles = queue_handle.await.into_diagnostic()?;

                if !job_handles.is_empty() {
                    debug!("Aborting running actions");

                    job_handles.shutdown().await;
                }
            }
        }

        self.actions = actions;
        self.duration = Some(start.elapsed());

        if let Some(error) = error {
            return Err(error);
        }

        Ok(())
    }

    #[instrument(skip_all)]
    fn dispatch_jobs(
        &self,
        action_graph: ActionGraph,
        job_context: JobContext,
        completed_queue: mpsc::UnboundedReceiver<CompletedJob>,
    ) -> miette::Result<JoinHandle<JobHandles>> {
        let node_indices = action_graph.sort_topological()?;
        let node_count = node_indices.len();
        let priority_groups = action_graph.group_priorities(node_indices);
        let app_context = Arc::clone(&self.app_context);
        let action_context = Arc::clone(&self.action_context);

        debug!(total_jobs = node_count, "Dispatching jobs in the pipeline");

        Ok(tokio::spawn(Box::pin(async move {
            // Always unblock the pipeline, even if dispatching fails
            let _drained = DrainedGuard(Arc::clone(&job_context.abort_state));

            let mut dispatcher = JobDispatcher::new(
                &action_graph,
                job_context.clone(),
                priority_groups,
                completed_queue,
            );
            let mut job_handles = JobHandles::default();
            let mut stopped = false;

            while dispatcher.has_queued_jobs() {
                // If the pipeline was aborted or cancelled (signal),
                // stop dispatching and abort all currently running handles
                if job_context.is_aborted_or_cancelled() {
                    stopped = true;
                    break;
                }

                // If none is returned, then we are waiting on other currently running
                // nodes to complete, but sometimes they cannot advance without
                // awaiting the current job handles. So to move this forward, only
                // advance 1 handle at a time!
                let Some(node_index) = dispatcher.next().await else {
                    job_handles.standard.join_next().await;

                    continue;
                };

                // Node does not exist for some reason, this shouldn't happen!
                let Some(node) = action_graph.get_node_from_index(&node_index) else {
                    warn!(
                        index = node_index.index(),
                        "Received action with no associated node, unable to dispatch job",
                    );

                    // Must mark as completed otherwise the loop hangs
                    job_context.mark_completed(node_index, false).await;

                    continue;
                };

                let is_cleanup = action_graph.is_cleanup_index(&node_index);

                // A cleanup job has nothing to clean up when none of the jobs it
                // cleans up after ran their command (they were skipped, or were
                // hydrated from the cache), unless it was explicitly requested
                if is_cleanup
                    && dispatcher.is_needless_cleanup(node_index)
                    && !matches!(node, ActionNode::RunTask(inner) if action_context.primary_targets.contains(&inner.target))
                {
                    debug!(
                        index = node_index.index(),
                        "Skipping cleanup job, as there's nothing to clean up"
                    );

                    let mut action = Action::new(node.to_owned());
                    action.node_index = node_index.index();
                    action.finish(ActionStatus::Skipped);

                    job_context.send_result(action).await;

                    continue;
                }

                // Persistent actions are dispatched topologically like any other
                // action, but they never complete on their own, so they require
                // some special handling
                if node.is_persistent() {
                    debug!(index = node_index.index(), "Dispatching persistent job");

                    // Mark as completed immediately, otherwise the loop hangs, and
                    // dependents (which must be persistent, or only wait on it to
                    // start) would never dispatch
                    job_context.mark_completed(node_index, true).await;

                    // Set the state early since it "never finishes", otherwise the
                    // runner will error about a missing hash if it's a dependency
                    // of another persistent task
                    if let ActionNode::RunTask(inner) = node {
                        action_context
                            .set_target_state(inner.target.clone(), TargetState::Passthrough);
                    }

                    // Dispatch without a permit, otherwise these long-running
                    // actions would consume the entire concurrency pool
                    job_handles.persistent.spawn(dispatch_job(
                        node.to_owned(),
                        node_index.index(),
                        is_cleanup,
                        job_context.clone(),
                        Arc::clone(&app_context),
                        Arc::clone(&action_context),
                    ));

                    continue;
                }

                // Other actions may only wait on this action to start, so run it
                // without a permit (like persistent actions), otherwise they may
                // never acquire one to run alongside it, and don't run it in
                // isolation, otherwise they would wait for it to complete
                if dispatcher.has_wait_dependents(node_index) {
                    job_handles.standard.spawn(dispatch_job(
                        node.to_owned(),
                        node_index.index(),
                        is_cleanup,
                        job_context.clone(),
                        Arc::clone(&app_context),
                        Arc::clone(&action_context),
                    ));

                    continue;
                }

                // Otherwise run the action topologically
                job_handles.standard.spawn(dispatch_job_with_permit(
                    node.to_owned(),
                    node_index.index(),
                    is_cleanup,
                    job_context.clone(),
                    Arc::clone(&app_context),
                    Arc::clone(&action_context),
                ));

                // Run this in isolation by exhausting the current list of handles.
                // Persistent handles are excluded as they never complete!
                if node.is_interactive()
                    && exhaust_job_handles(&mut job_handles.standard, &job_context).await
                {
                    stopped = true;
                    break;
                }
            }

            if !stopped {
                // Ensure all non-persistent actions have finished, while allowing
                // persistent actions to continue running in the background
                exhaust_job_handles(&mut job_handles.standard, &job_context).await;
            }

            // A failure may have aborted the pipeline, but the jobs that clean up
            // after the jobs that have ran must still run
            if job_context.abort_token.is_cancelled()
                && !action_graph.get_cleanup_indices().is_empty()
            {
                drain_cleanup_jobs(
                    &mut dispatcher,
                    &mut job_handles,
                    &action_graph,
                    &job_context,
                    &app_context,
                    &action_context,
                )
                .await;
            }

            job_handles
        })))
    }

    fn monitor_signals(
        &self,
        cancel_token: CancellationToken,
        abort_state: Arc<AbortState>,
    ) -> JoinHandle<SignalType> {
        // Subscribe before spawning, so that no signals are missed, including
        // the pipeline's own termination, which must be accounted for
        let mut receiver = ProcessRegistry::instance().receive_signal();

        tokio::spawn(async move {
            loop {
                match receiver.recv().await {
                    Ok(signal) => {
                        // Aborting the pipeline terminates running processes, which is
                        // broadcast like a signal, but isn't one, so keep listening
                        if matches!(signal, SignalType::Terminate) && abort_state.take_terminated()
                        {
                            continue;
                        }

                        cancel_token.cancel();

                        debug!("Received signal, shutting down pipeline");

                        return signal;
                    }
                    Err(RecvError::Lagged(_)) => continue,
                    Err(RecvError::Closed) => break,
                };
            }

            SignalType::Interrupt
        })
    }

    async fn setup_subscribers(&mut self) {
        debug!("Registering event subscribers");

        if !self.quiet {
            self.emitter
                .subscribe(ConsoleSubscriber::new(
                    self.app_context.console.clone(),
                    self.summary,
                ))
                .await;
        }

        debug!("Subscribing run reports and estimates");

        self.emitter
            .subscribe(ReportsSubscriber::new(
                Arc::clone(&self.app_context.cache_engine),
                Arc::clone(&self.action_context),
                &self.report_name,
            ))
            .await;

        // For security and privacy purposes, only send webhooks in a remote environment
        if (is_remote() || is_test_env())
            && let Some(webhook_url) = &self.app_context.workspace_config.notifier.webhook_url
        {
            let require_acknowledge = self
                .app_context
                .workspace_config
                .notifier
                .webhook_acknowledge;

            debug!(
                url = webhook_url,
                "Subscribing webhook events ({} enabled)",
                color::property("notifier.webhookUrl"),
            );

            self.emitter
                .subscribe(WebhooksSubscriber::new(
                    webhook_url,
                    require_acknowledge,
                    self.daemon_client.clone(),
                ))
                .await;
        }

        if let Some(toast) = self
            .app_context
            .workspace_config
            .notifier
            .terminal_notifications
        {
            debug!(
                "Subscribing terminal notifications ({} enabled)",
                color::property("notifier.terminalNotifications"),
            );

            self.emitter
                .subscribe(NotificationsSubscriber::new(toast))
                .await;
        }

        if self.app_context.workspace_config.pipeline.auto_clean_cache {
            let lifetime = &self.app_context.workspace_config.pipeline.cache_lifetime;

            debug!(
                lifetime = lifetime,
                "Subscribing cache cleanup ({} enabled)",
                color::property("pipeline.autoCleanCache"),
            );

            self.emitter
                .subscribe(CleanupSubscriber::new(
                    Arc::clone(&self.app_context.cache_engine),
                    self.daemon_client.clone(),
                    lifetime,
                ))
                .await;
        }

        // Metrics are recorded against the global meter provider, which is
        // only configured when OTLP exporting has been enabled
        if self.app_context.otel_enabled {
            debug!(
                "Subscribing OpenTelemetry metrics ({} enabled)",
                color::property("--otel"),
            );

            self.emitter
                .subscribe(MetricsSubscriber::new(Arc::clone(&self.workspace_graph)))
                .await;
        }

        // TODO: Disabled for now as we've hit the posthog limit!
        // if self.app_context.workspace_config.telemetry {
        //     debug!("Subscribing telemetry");

        //     self.emitter
        //         .subscribe(TelemetrySubscriber::new(Arc::clone(
        //             &self.app_context.toolchains_config,
        //         )))
        //         .await;
        // }
    }
}

#[instrument(skip(job_context, app_context, action_context))]
async fn dispatch_job(
    node: ActionNode,
    node_index: usize,
    cleanup: bool,
    job_context: JobContext,
    app_context: Arc<AppContext>,
    action_context: Arc<ActionContext>,
) {
    let job = Job {
        node,
        node_index,
        cleanup,
        context: job_context,
        app_context,
        action_context,
    };

    job.dispatch().await;
}

async fn dispatch_job_with_permit(
    node: ActionNode,
    node_index: usize,
    cleanup: bool,
    job_context: JobContext,
    app_context: Arc<AppContext>,
    action_context: Arc<ActionContext>,
) {
    // Cleanup jobs that run after an abort must wait before they can start,
    // so don't hold a permit that other jobs could use while waiting
    if cleanup && job_context.abort_token.is_cancelled() {
        job_context
            .abort_state
            .wait_until_resumable(&job_context.cancel_token)
            .await;
    }

    let permit = job_context
        .semaphore
        .clone()
        .acquire_owned()
        .await
        .expect("Failed to dispatch job!");

    dispatch_job(
        node,
        node_index,
        cleanup,
        job_context,
        app_context,
        action_context,
    )
    .await;

    drop(permit);
}

/// Run the cleanup jobs for the jobs that ran before the pipeline was aborted,
/// as they must always run after them, even if they failed. Jobs that never
/// ran have nothing to clean up, and cleanup jobs aren't ran after a signal.
#[instrument(skip_all)]
async fn drain_cleanup_jobs(
    dispatcher: &mut JobDispatcher<'_>,
    job_handles: &mut JobHandles,
    action_graph: &ActionGraph,
    job_context: &JobContext,
    app_context: &Arc<AppContext>,
    action_context: &Arc<ActionContext>,
) {
    // Wait for the pipeline to terminate running processes,
    // and to determine whether cleanup jobs should run
    tokio::select! {
        _ = job_context.abort_state.wait_until_handled() => {}
        _ = job_context.cancel_token.cancelled() => {}
    };

    if !job_context.abort_state.should_drain() || job_context.cancel_token.is_cancelled() {
        return;
    }

    debug!("Pipeline was aborted, running cleanup jobs for the jobs that have ran");

    // Jobs that were running when the pipeline was aborted are being terminated,
    // but they're only waited on when cleanup jobs relate to them, and only until
    // they should have been terminated. Unrelated jobs are left to the pipeline
    let give_up_at = job_context.abort_state.get_give_up_at();

    // Cleanup jobs may relate to each other, so continue
    // until nothing else can run, and nothing is running
    while !job_context.cancel_token.is_cancelled() {
        let give_up = give_up_at.is_some_and(|at| Instant::now() >= at);

        for index in dispatcher.take_cleanups_after_abort(give_up) {
            let Some(node) = action_graph.get_node_from_index(&index) else {
                continue;
            };

            debug!(index = index.index(), "Dispatching cleanup job");

            if dispatcher.has_wait_dependents(index) {
                job_handles.standard.spawn(dispatch_job(
                    node.to_owned(),
                    index.index(),
                    true,
                    job_context.clone(),
                    Arc::clone(app_context),
                    Arc::clone(action_context),
                ));
            } else {
                job_handles.standard.spawn(dispatch_job_with_permit(
                    node.to_owned(),
                    index.index(),
                    true,
                    job_context.clone(),
                    Arc::clone(app_context),
                    Arc::clone(action_context),
                ));
            }
        }

        if !dispatcher.is_waiting_on_cleanups(give_up) || job_handles.standard.is_empty() {
            break;
        }

        // Wait for a job to complete, which may unblock cleanup jobs,
        // or until we give up on the jobs that are being terminated
        let give_up_timer = async {
            match give_up_at {
                Some(at) if !give_up => tokio::time::sleep_until(at.into()).await,
                _ => std::future::pending().await,
            }
        };

        tokio::select! {
            _ = job_handles.standard.join_next() => {}
            _ = job_context.cancel_token.cancelled() => break,
            _ = give_up_timer => {}
        };
    }
}

#[instrument(skip_all)]
async fn exhaust_job_handles<T: 'static>(set: &mut JoinSet<T>, job_context: &JobContext) -> bool {
    while set.join_next().await.is_some() {
        continue;
    }

    set.detach_all();

    job_context.is_aborted_or_cancelled()
}
