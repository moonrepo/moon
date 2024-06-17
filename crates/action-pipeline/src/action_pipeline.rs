use crate::job::Job;
use crate::job_context::JobContext;
use moon_action::{Action, ActionNode, ActionStatus};
use moon_action_context::ActionContext;
use moon_action_graph::ActionGraph;
use moon_app_context::AppContext;
use moon_project_graph::ProjectGraph;
use std::mem;
use std::sync::Arc;
use tokio::sync::{mpsc, OwnedSemaphorePermit, Semaphore};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::{debug, trace, warn};

pub struct ActionPipeline {
    app_context: Arc<AppContext>,
    project_graph: Arc<ProjectGraph>,

    bail: bool,
    concurrency: Option<usize>,
}

impl ActionPipeline {
    pub fn new(app_context: Arc<AppContext>, project_graph: Arc<ProjectGraph>) -> Self {
        Self {
            app_context,
            project_graph,
            bail: false,
            concurrency: None,
        }
    }

    pub async fn run(&self, action_graph: ActionGraph) -> miette::Result<Vec<Action>> {
        self.run_with_context(action_graph, ActionContext::default())
            .await
    }

    pub async fn run_with_context(
        &self,
        action_graph: ActionGraph,
        action_context: ActionContext,
    ) -> miette::Result<Vec<Action>> {
        let concurrency = self.concurrency.unwrap_or_else(num_cpus::get);
        let total_actions = action_graph.get_node_count();

        // TODO pipeline started event

        self.app_context
            .console
            .reporter
            .on_pipeline_started(&action_graph.get_nodes())?;

        // This aggregates results from ran jobs
        let (sender, mut receiver) = mpsc::channel::<Action>(total_actions);

        // Create job context
        let job_context = Arc::new(JobContext {
            abort_token: CancellationToken::new(),
            cancel_token: CancellationToken::new(),
            project_graph: Arc::clone(&self.project_graph),
            result_sender: sender.clone(),
            semaphore: Arc::new(Semaphore::new(concurrency)),
        });

        // Monitor signals and ctrl+c
        let signal_handle = self.monitor_signals(job_context.cancel_token.clone());

        // Enqueue jobs from the graph to dispatch actions
        let queue_handle = self.enqueue_jobs(
            action_graph,
            Arc::new(action_context),
            Arc::clone(&job_context),
        )?;

        // Wait and receive all results coming through
        let mut actions = vec![];
        let mut ran_actions = 0;

        while let Some(action) = receiver.recv().await {
            ran_actions += 1;
            actions.push(action);

            if ran_actions == total_actions || job_context.is_aborted_or_cancelled() {
                break;
            }
        }

        drop(sender);
        drop(receiver);

        // Clean up any open handles
        queue_handle.abort();
        signal_handle.abort();

        Ok(actions)
    }

    fn enqueue_jobs(
        &self,
        action_graph: ActionGraph,
        action_context: Arc<ActionContext>,
        job_context: Arc<JobContext>,
    ) -> miette::Result<JoinHandle<()>> {
        let node_indices = action_graph.sort_topological()?;
        let app_context = Arc::clone(&self.app_context);

        Ok(tokio::spawn(async move {
            let mut nodes = action_graph.creater_iter(node_indices);
            let mut persistent_indices = vec![];
            let mut job_handles = vec![];

            nodes.monitor_completed();

            while nodes.has_pending() {
                if job_context.is_aborted_or_cancelled() {
                    break;
                }

                // Nothing new to run since they're waiting on currently
                // running actions, so just keep looping
                let Some(node_index) = nodes.next() else {
                    continue;
                };

                // Node does not exist for some reason, this shouldn't happen!
                let Some(node) = action_graph.get_node_from_index(&node_index) else {
                    warn!(
                        "Received action {} with no associated node, unable to dispatch",
                        node_index.index()
                    );

                    continue;
                };

                // Run persistent later in parallel
                if node.is_persistent() {
                    trace!(
                        "Marking action {} as persistent, will defer dispatch",
                        node_index.index()
                    );

                    // Must mark as completed otherwise the loop hangs
                    nodes.mark_completed(node_index);
                    persistent_indices.push(node_index);

                    continue;
                }

                // Otherwise run it topologically
                job_handles.push(dispatch_job(
                    node.to_owned(),
                    node_index.index(),
                    Arc::clone(&job_context),
                    Arc::clone(&app_context),
                    Arc::clone(&action_context),
                ));

                // Run this in isolation by exhausting the current list of handles
                if node.is_interactive() {
                    for handle in mem::take(&mut job_handles) {
                        handle.await;
                    }
                }
            }

            // TODO persistent
        }))
    }

    async fn handle_jobs(&self, jobs: Vec<JoinHandle<()>>) -> miette::Result<()> {
        // TODO exhaust them
        Ok(())
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

async fn dispatch_job(
    node: ActionNode,
    index: usize,
    job_context: Arc<JobContext>,
    app_context: Arc<AppContext>,
    action_context: Arc<ActionContext>,
) -> JoinHandle<ActionStatus> {
    let permit = job_context
        .semaphore
        .clone()
        .acquire_owned()
        .await
        .expect("Failed to dispatch job!");

    let job = Job {
        node,
        index,
        context: job_context,
        app_context,
        action_context,
        timeout: None, // TODO
    };

    tokio::spawn(async move {
        let status = job.dispatch().await;
        drop(permit);
        status
    })
}
