use crate::job::Job;
use crate::job_context::JobContext;
use crate::job_dispatcher::JobDispatcher;
use moon_action::{Action, ActionNode};
use moon_action_context::ActionContext;
use moon_action_graph::ActionGraph;
use moon_app_context::AppContext;
use moon_project_graph::ProjectGraph;
use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock, Semaphore};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::{debug, instrument, trace, warn};

pub struct ActionPipeline {
    pub bail: bool,
    pub concurrency: Option<usize>,
    pub summarize: bool,

    app_context: Arc<AppContext>,
    project_graph: Arc<ProjectGraph>,
}

impl ActionPipeline {
    pub fn new(app_context: Arc<AppContext>, project_graph: Arc<ProjectGraph>) -> Self {
        debug!("Creating pipeline to run actions");

        Self {
            app_context,
            project_graph,
            bail: false,
            concurrency: None,
            summarize: false,
        }
    }

    pub async fn run(&self, action_graph: ActionGraph) -> miette::Result<Vec<Action>> {
        self.run_with_context(action_graph, ActionContext::default())
            .await
    }

    #[instrument(name = "run_pipeline", skip_all)]
    pub async fn run_with_context(
        &self,
        action_graph: ActionGraph,
        action_context: ActionContext,
    ) -> miette::Result<Vec<Action>> {
        let concurrency = self.concurrency.unwrap_or_else(num_cpus::get);
        let total_actions = action_graph.get_node_count();

        debug!(total_actions, concurrency, "Starting pipeline");

        // TODO pipeline started event

        self.app_context
            .console
            .reporter
            .on_pipeline_started(&action_graph.get_nodes())?;

        // This aggregates results from jobs
        let (sender, mut receiver) = mpsc::channel::<Action>(total_actions);

        // Create job context
        let abort_token = CancellationToken::new();
        let cancel_token = CancellationToken::new();

        let job_context = JobContext {
            abort_token: abort_token.clone(),
            cancel_token: cancel_token.clone(),
            completed_jobs: Arc::new(RwLock::new(FxHashSet::default())),
            project_graph: Arc::clone(&self.project_graph),
            result_sender: sender,
            semaphore: Arc::new(Semaphore::new(concurrency)),
            running_jobs: Arc::new(RwLock::new(FxHashMap::default())),
        };

        // Monitor signals and ctrl+c
        let signal_handle = self.monitor_signals(cancel_token.clone());

        // Dispatch jobs from the graph to run actions
        let queue_handle = self.dispatch_jobs(action_graph, action_context, job_context)?;

        // Wait and receive all results coming through
        debug!("Waiting for jobs to return results");

        let mut actions = vec![];
        let mut ran_actions = 0;

        while let Some(action) = receiver.recv().await {
            ran_actions += 1;

            if self.bail && action.should_bail() || action.should_abort() {
                abort_token.cancel();
            }

            actions.push(action);

            if ran_actions == total_actions {
                debug!("Finished pipeline, received all results");
                break;
            } else if abort_token.is_cancelled() {
                debug!("Aborting pipeline (because something failed)");
                break;
            } else if cancel_token.is_cancelled() {
                debug!("Cancelling pipeline (via signal)");
                break;
            }
        }

        drop(receiver);

        // Clean up any open handles
        queue_handle.abort();
        signal_handle.abort();

        Ok(actions)
    }

    #[instrument(skip_all)]
    fn dispatch_jobs(
        &self,
        action_graph: ActionGraph,
        action_context: ActionContext,
        job_context: JobContext,
    ) -> miette::Result<JoinHandle<()>> {
        let node_indices = action_graph.sort_topological()?;
        let app_context = Arc::clone(&self.app_context);
        let action_context = Arc::new(action_context);

        debug!(
            total_jobs = node_indices.len(),
            "Dispatching jobs in the pipeline"
        );

        Ok(tokio::spawn(async move {
            let mut dispatcher =
                JobDispatcher::new(&action_graph, job_context.clone(), node_indices);
            let mut persistent_indices = vec![];
            let mut job_handles = VecDeque::<JoinHandle<()>>::new();

            while dispatcher.has_queued_jobs() {
                // If the pipeline was aborted or cancelled (signal),
                // loop through and abort all currently running handles
                if job_context.is_aborted_or_cancelled() {
                    for handle in job_handles {
                        if !handle.is_finished() {
                            handle.abort();
                        }
                    }

                    // Return instead of break, so that we avoid
                    // running persistent tasks below
                    return;
                }

                // If none is returned, then we are waiting on other currently running
                // nodes to complete, but sometimes they cannot advance without
                // awaiting the current job handles. So to move this forward, only
                // advance 1 handle at a time!
                let Some(node_index) = dispatcher.next().await else {
                    if let Some(handle) = job_handles.pop_front() {
                        let _ = handle.await;
                    }

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

                // Run persistent actions later in parallel, so only grab the index for now
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
                job_handles.push_back(
                    dispatch_job(
                        node.to_owned(),
                        node_index.index(),
                        job_context.clone(),
                        Arc::clone(&app_context),
                        Arc::clone(&action_context),
                    )
                    .await,
                );

                // Run this in isolation by exhausting the current list of handles
                if node.is_interactive() {
                    for handle in job_handles.drain(0..) {
                        let _ = handle.await;
                    }
                }
            }

            if !persistent_indices.is_empty() {
                debug!("Running {} persistent actions", persistent_indices.len());

                for node_index in persistent_indices {
                    job_handles.push_back(
                        dispatch_job(
                            action_graph
                                .get_node_from_index(&node_index)
                                .unwrap()
                                .to_owned(),
                            node_index.index(),
                            job_context.clone(),
                            Arc::clone(&app_context),
                            Arc::clone(&action_context),
                        )
                        .await,
                    );
                }
            }

            // Run any remaining actions
            for handle in job_handles {
                let _ = handle.await;
            }
        }))
    }

    fn monitor_signals(&self, cancel_token: CancellationToken) -> JoinHandle<()> {
        tokio::spawn(async move {
            debug!("Listening for ctrl+c signal");

            if tokio::signal::ctrl_c().await.is_ok() {
                debug!("Received ctrl+c signal, shutting down!");

                cancel_token.cancel();
            }
        })
    }
}

#[instrument(skip(job_context, app_context, action_context))]
async fn dispatch_job(
    node: ActionNode,
    node_index: usize,
    job_context: JobContext,
    app_context: Arc<AppContext>,
    action_context: Arc<ActionContext>,
) -> JoinHandle<()> {
    let permit = job_context
        .semaphore
        .clone()
        .acquire_owned()
        .await
        .expect("Failed to dispatch job!");

    let job = Job {
        timeout: match &node {
            ActionNode::RunTask(inner) => inner.timeout,
            _ => None,
        },
        node,
        node_index,
        context: job_context,
        app_context,
        action_context,
    };

    tokio::spawn(async move {
        job.dispatch().await;
        drop(permit);
    })
}
