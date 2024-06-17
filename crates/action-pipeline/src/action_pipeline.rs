use crate::job::Job;
use crate::job_context::JobContext;
use moon_action::{Action, ActionNode};
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

    pub async fn run(&self, action_graph: ActionGraph) -> miette::Result<()> {
        self.run_with_context(action_graph, ActionContext::default())
            .await
    }

    pub async fn run_with_context(
        &self,
        action_graph: ActionGraph,
        action_context: ActionContext,
    ) -> miette::Result<()> {
        let concurrency = self.concurrency.unwrap_or_else(num_cpus::get);

        // TODO pipeline started event

        self.app_context
            .console
            .reporter
            .on_pipeline_started(&action_graph.get_nodes())?;

        // This aggregates results from ran jobs
        let (sender, mut receiver) = mpsc::channel::<Action>(action_graph.get_node_count());

        // Create job context
        let job_context = JobContext {
            abort_token: CancellationToken::new(),
            cancel_token: CancellationToken::new(),
            semaphore: Arc::new(Semaphore::new(concurrency)),
            result_sender: sender.clone(),
        };

        // Monitor signals and ctrl+c
        let signal_handle = self.monitor_signals(job_context.cancel_token.clone());

        // Generate jobs to process actions
        self.generate_jobs(action_graph, action_context, job_context)
            .await?;

        // Clean up any open handles
        signal_handle.abort();

        Ok(())
    }

    async fn generate_jobs(
        &self,
        action_graph: ActionGraph,
        action_context: ActionContext,
        job_context: JobContext,
    ) -> miette::Result<()> {
        let action_context = Arc::new(action_context);
        let mut nodes = action_graph.try_iter()?;
        let mut persistent_indexes = vec![];
        let mut jobs = vec![];

        nodes.monitor_completed();

        let create_job = |node: &ActionNode, permit: OwnedSemaphorePermit| Job {
            node: node.to_owned(),
            context: job_context.clone(),
            app_context: Arc::clone(&self.app_context),
            action_context: Arc::clone(&action_context),
            permit,
            timeout: None,
        };

        while nodes.has_pending() {
            if job_context.is_aborted_or_cancelled() {
                break;
            }

            // Nothing new to run since they're waiting on currently
            // running actions, so exhaust the current list
            let Some(node_index) = nodes.next() else {
                self.dispatch_jobs(mem::take(&mut jobs)).await?;

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
                persistent_indexes.push(node_index);

                continue;
            }

            // Otherwise run it topologically
            jobs.push(create_job(
                node,
                job_context
                    .semaphore
                    .clone()
                    .acquire_owned()
                    .await
                    .expect("Failed to spawn job!"),
            ));

            // Run this in isolation by exhausting the current list of handles
            if node.is_interactive() || job_context.semaphore.available_permits() == 0 {
                self.dispatch_jobs(mem::take(&mut jobs)).await?;
            }
        }

        // TODO persistent

        Ok(())
    }

    async fn dispatch_jobs(&self, jobs: Vec<Job>) -> miette::Result<()> {
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
