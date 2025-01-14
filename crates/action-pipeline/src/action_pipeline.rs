use crate::event_emitter::{Event, EventEmitter};
use crate::job::Job;
use crate::job_context::JobContext;
use crate::job_dispatcher::JobDispatcher;
use crate::subscribers::cleanup_subscriber::CleanupSubscriber;
use crate::subscribers::console_subscriber::ConsoleSubscriber;
use crate::subscribers::moonbase_subscriber::MoonbaseSubscriber;
use crate::subscribers::remote_subscriber::RemoteSubscriber;
use crate::subscribers::reports_subscriber::ReportsSubscriber;
use crate::subscribers::webhooks_subscriber::WebhooksSubscriber;
use moon_action::{Action, ActionNode};
use moon_action_context::{ActionContext, TargetState};
use moon_action_graph::ActionGraph;
use moon_api::Moonbase;
use moon_app_context::AppContext;
use moon_common::{color, is_ci, is_test_env};
use moon_process::ProcessRegistry;
use moon_toolchain_plugin::ToolchainRegistry;
use moon_workspace_graph::WorkspaceGraph;
use rustc_hash::{FxHashMap, FxHashSet};
use std::mem;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, RwLock, Semaphore};
use tokio::task::{JoinHandle, JoinSet};
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;
use tracing::{debug, instrument, trace, warn};

pub struct ActionPipeline {
    pub bail: bool,
    pub concurrency: usize,
    pub report_name: String,
    pub summarize: bool,

    // State
    aborted: bool,
    actions: Vec<Action>,
    duration: Option<Duration>,

    // Data
    app_context: Arc<AppContext>,
    action_context: Arc<ActionContext>,
    emitter: Arc<EventEmitter>,
    toolchain_registry: Arc<ToolchainRegistry>,
    workspace_graph: WorkspaceGraph,
}

impl ActionPipeline {
    pub fn new(
        app_context: Arc<AppContext>,
        toolchain_registry: Arc<ToolchainRegistry>,
        workspace_graph: WorkspaceGraph,
    ) -> Self {
        debug!("Creating pipeline to run actions");

        Self {
            aborted: false,
            actions: vec![],
            action_context: Arc::new(ActionContext::default()),
            app_context,
            bail: false,
            concurrency: num_cpus::get(),
            duration: None,
            emitter: Arc::new(EventEmitter::default()),
            report_name: "runReport.json".into(),
            summarize: false,
            toolchain_registry,
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
                        aborted: self.aborted,
                        context: &self.action_context,
                        duration: self.duration,
                        error: None,
                        error_report: None,
                    })
                    .await?;

                Ok(actions)
            }
            Err(error) => {
                self.emitter
                    .emit(Event::PipelineCompleted {
                        actions: &actions,
                        aborted: self.aborted,
                        context: &self.action_context,
                        duration: self.duration,
                        error: Some(error.to_string()),
                        error_report: Some(&error),
                    })
                    .await?;

                Err(error)
            }
        }
    }

    pub async fn internal_run(&mut self, action_graph: ActionGraph) -> miette::Result<()> {
        let total_actions = action_graph.get_node_count();
        let start = Instant::now();

        debug!(
            total_actions,
            concurrency = self.concurrency,
            "Starting pipeline"
        );

        // This aggregates results from jobs
        let (sender, mut receiver) = mpsc::channel::<Action>(total_actions.max(1));

        // Create job context
        let abort_token = CancellationToken::new();
        let cancel_token = CancellationToken::new();

        let job_context = JobContext {
            abort_token: abort_token.clone(),
            cancel_token: cancel_token.clone(),
            completed_jobs: Arc::new(RwLock::new(FxHashSet::default())),
            emitter: Arc::clone(&self.emitter),
            result_sender: sender,
            semaphore: Arc::new(Semaphore::new(self.concurrency)),
            running_jobs: Arc::new(RwLock::new(FxHashMap::default())),
            toolchain_registry: Arc::clone(&self.toolchain_registry),
            workspace_graph: self.workspace_graph.clone(),
        };

        // Monitor signals and ctrl+c
        let signal_handle = self.monitor_signals(cancel_token.clone());

        // Dispatch jobs from the graph to run actions
        let queue_handle = self.dispatch_jobs(action_graph, job_context)?;

        // Wait and receive all results coming through
        debug!("Waiting for jobs to return results");

        let process_registry = ProcessRegistry::instance();
        let mut actions = vec![];
        let mut error = None;

        while let Some(mut action) = receiver.recv().await {
            if self.bail && action.should_bail() || action.should_abort() {
                process_registry.terminate_running();
                abort_token.cancel();
                error = Some(action.get_error());
            }

            actions.push(action);

            if abort_token.is_cancelled() {
                debug!("Aborting pipeline (because something failed)");
                break;
            } else if cancel_token.is_cancelled() {
                debug!("Cancelling pipeline (via signal)");
                break;
            } else if actions.len() == total_actions {
                debug!("Finished pipeline, received all results");
                break;
            }
        }

        drop(receiver);

        // Wait for the queue to abort/close all running tasks
        let _ = queue_handle.await;
        process_registry.wait_for_running_to_shutdown().await;

        // Force abort the signal handler
        signal_handle.abort();

        self.aborted = abort_token.is_cancelled();
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
    ) -> miette::Result<JoinHandle<()>> {
        let node_indices = action_graph.sort_topological()?;
        let app_context = Arc::clone(&self.app_context);
        let action_context = Arc::clone(&self.action_context);

        debug!(
            total_jobs = node_indices.len(),
            "Dispatching jobs in the pipeline"
        );

        Ok(tokio::spawn(async move {
            let mut dispatcher =
                JobDispatcher::new(&action_graph, job_context.clone(), node_indices);
            let mut persistent_indices = vec![];
            let mut job_handles = JoinSet::new();

            while dispatcher.has_queued_jobs() {
                // If the pipeline was aborted or cancelled (signal),
                // loop through and abort all currently running handles
                if job_context.is_aborted_or_cancelled() {
                    job_handles.shutdown().await;

                    // Return instead of break, so that we avoid
                    // running persistent tasks below
                    return;
                }

                // If none is returned, then we are waiting on other currently running
                // nodes to complete, but sometimes they cannot advance without
                // awaiting the current job handles. So to move this forward, only
                // advance 1 handle at a time!
                let Some(node_index) = dispatcher.next().await else {
                    job_handles.join_next().await;

                    continue;
                };

                // Node does not exist for some reason, this shouldn't happen!
                let Some(node) = action_graph.get_node_from_index(&node_index) else {
                    warn!(
                        index = node_index.index(),
                        "Received action with no associated node, unable to dispatch job",
                    );

                    // Must mark as completed otherwise the loop hangs
                    job_context.mark_completed(node_index).await;

                    continue;
                };

                // Run persistent actions later, so only grab the index for now
                if node.is_persistent() {
                    trace!(
                        index = node_index.index(),
                        "Marking action as persistent, will defer dispatch",
                    );

                    // Must mark as completed otherwise the loop hangs
                    job_context.mark_completed(node_index).await;
                    persistent_indices.push(node_index);

                    continue;
                }

                // Otherwise run the action topologically
                job_handles.spawn(dispatch_job_with_permit(
                    node.to_owned(),
                    node_index.index(),
                    job_context.clone(),
                    Arc::clone(&app_context),
                    Arc::clone(&action_context),
                ));

                // Run this in isolation by exhausting the current list of handles
                if node.is_interactive()
                    && exhaust_job_handles(&mut job_handles, &job_context).await
                {
                    return;
                }
            }

            // Ensure all non-persistent actions have finished
            if exhaust_job_handles(&mut job_handles, &job_context).await {
                return;
            }

            // Then run all persistent actions in parallel
            if persistent_indices.is_empty() {
                return;
            }

            debug!(
                indices = ?persistent_indices,
                "Running {} persistent actions",
                persistent_indices.len()
            );

            persistent_indices
                .into_iter()
                .flat_map(|node_index| {
                    let node = action_graph.get_node_from_index(&node_index)?;

                    // Since the task is persistent, set the state early since
                    // it "never finishes", otherwise the runner will error about
                    // a missing hash if it's a dependency of another persistent task
                    if let ActionNode::RunTask(inner) = node {
                        action_context
                            .set_target_state(inner.target.clone(), TargetState::Passthrough);
                    }

                    Some((node.to_owned(), node_index.index()))
                })
                .for_each(|(node, node_index)| {
                    job_handles.spawn(dispatch_job(
                        node,
                        node_index,
                        job_context.clone(),
                        Arc::clone(&app_context),
                        Arc::clone(&action_context),
                    ));
                });

            // Since these tasks are persistent and never complete,
            // we need to continually check if they've been aborted or
            // cancelled, otherwise we will end up with zombie processes
            loop {
                sleep(Duration::from_millis(50)).await;

                // No tasks running, so don't hang forever
                if job_context.result_sender.is_closed() {
                    break;
                }

                if job_context.is_aborted_or_cancelled() {
                    debug!("Shutting down {} persistent jobs", job_handles.len());

                    job_handles.shutdown().await;
                    break;
                }
            }
        }))
    }

    fn monitor_signals(&self, cancel_token: CancellationToken) -> JoinHandle<()> {
        tokio::spawn(async move {
            let mut receiver = ProcessRegistry::instance().receive_signal();

            if receiver.recv().await.is_ok() {
                cancel_token.cancel();
            }
        })
    }

    async fn setup_subscribers(&mut self) {
        debug!("Registering event subscribers");

        self.emitter
            .subscribe(ConsoleSubscriber::new(
                Arc::clone(&self.app_context.console),
                self.summarize,
            ))
            .await;

        self.emitter.subscribe(RemoteSubscriber).await;

        debug!("Subscribing run reports and estimates");

        self.emitter
            .subscribe(ReportsSubscriber::new(
                Arc::clone(&self.app_context.cache_engine),
                Arc::clone(&self.action_context),
                &self.report_name,
            ))
            .await;

        if let Some(session) = Moonbase::session() {
            debug!("Subscribing moonbase");

            self.emitter
                .subscribe(MoonbaseSubscriber::new(session))
                .await;
        }

        // For security and privacy purposes, only send webhooks from a CI environment
        if is_ci() || is_test_env() {
            if let Some(webhook_url) = &self.app_context.workspace_config.notifier.webhook_url {
                debug!(
                    url = webhook_url,
                    "Subscribing webhook events ({} enabled)",
                    color::property("notifier.webhookUrl"),
                );

                self.emitter
                    .subscribe(WebhooksSubscriber::new(webhook_url))
                    .await;
            }
        }

        if self.app_context.workspace_config.runner.auto_clean_cache {
            let lifetime = &self.app_context.workspace_config.runner.cache_lifetime;

            debug!(
                lifetime = lifetime,
                "Subscribing cache cleanup ({} enabled)",
                color::property("runner.autoCleanCache"),
            );

            self.emitter
                .subscribe(CleanupSubscriber::new(
                    Arc::clone(&self.app_context.cache_engine),
                    lifetime,
                ))
                .await;
        }
    }
}

#[instrument(skip(job_context, app_context, action_context))]
async fn dispatch_job(
    node: ActionNode,
    node_index: usize,
    job_context: JobContext,
    app_context: Arc<AppContext>,
    action_context: Arc<ActionContext>,
) {
    let job = Job {
        node,
        node_index,
        context: job_context,
        app_context,
        action_context,
    };

    job.dispatch().await;
}

async fn dispatch_job_with_permit(
    node: ActionNode,
    node_index: usize,
    job_context: JobContext,
    app_context: Arc<AppContext>,
    action_context: Arc<ActionContext>,
) {
    let permit = job_context
        .semaphore
        .clone()
        .acquire_owned()
        .await
        .expect("Failed to dispatch job!");

    dispatch_job(node, node_index, job_context, app_context, action_context).await;

    drop(permit);
}

#[instrument(skip_all)]
async fn exhaust_job_handles<T: 'static>(set: &mut JoinSet<T>, job_context: &JobContext) -> bool {
    while set.join_next().await.is_some() {
        // If the pipeline was aborted or cancelled (signal),
        // loop through and abort all currently running handles
        if job_context.is_aborted_or_cancelled() {
            set.shutdown().await;
            set.detach_all();

            // Aborted
            return true;
        }
    }

    set.detach_all();

    // Not aborted
    false
}
