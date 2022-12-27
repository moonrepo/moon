use crate::errors::PipelineError;
use crate::processor::process_action;
use crate::subscribers::local_cache::LocalCacheSubscriber;
use crate::subscribers::moonbase_cache::MoonbaseCacheSubscriber;
use moon_action::{Action, ActionNode};
use moon_action_context::ActionContext;
use moon_dep_graph::DepGraph;
use moon_emitter::{Emitter, Event};
use moon_logger::{color, debug, error, trace};
use moon_notifier::WebhooksSubscriber;
use moon_project_graph::ProjectGraph;
use moon_utils::{is_ci, is_test_env};
use moon_workspace::Workspace;
use std::num::NonZeroUsize;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use tokio::sync::{OwnedSemaphorePermit, RwLock, Semaphore};

const LOG_TARGET: &str = "moon:action-pipeline";

pub type ActionResults = Vec<Action>;

pub struct Pipeline {
    bail: bool,

    concurrency: usize,

    duration: Option<Duration>,

    project_graph: Arc<RwLock<ProjectGraph>>,

    report_name: Option<String>,

    workspace: Arc<RwLock<Workspace>>,
}

impl Pipeline {
    pub fn new(workspace: Workspace, project_graph: ProjectGraph) -> Self {
        let concurrency = thread::available_parallelism()
            .unwrap_or(NonZeroUsize::new(8).unwrap())
            .get();

        Pipeline {
            bail: false,
            concurrency,
            duration: None,
            project_graph: Arc::new(RwLock::new(project_graph)),
            report_name: None,
            workspace: Arc::new(RwLock::new(workspace)),
        }
    }

    pub fn bail_on_error(&mut self) -> &mut Self {
        self.bail = true;
        self
    }

    pub fn concurrency(&mut self, value: usize) -> &Self {
        self.concurrency = value;
        self
    }

    pub fn generate_report(&mut self, name: &str) -> &mut Self {
        self.report_name = Some(name.to_owned());
        self
    }

    pub async fn run(
        &mut self,
        dep_graph: DepGraph,
        context: Option<ActionContext>,
    ) -> Result<(), PipelineError> {
        let start = Instant::now();
        let context = Arc::new(RwLock::new(context.unwrap_or_default()));
        let emitter = Arc::new(RwLock::new(
            create_emitter(Arc::clone(&self.workspace)).await,
        ));

        // We use an async channel to coordinate actions (tasks) to process
        // across a bounded worker pool, as defined by the provided concurrency
        let (sender, receiver) = async_channel::unbounded::<(Action, OwnedSemaphorePermit)>();
        let local_emitter = emitter.read().await;
        let mut results: ActionResults = vec![];
        let mut passed_count = 0;
        let mut cached_count = 0;
        let mut failed_count = 0;
        let mut abort: Option<String> = None;

        // Spawn worker threads that will process the action queue
        for _ in 0..self.concurrency {
            let receiver = receiver.clone();
            let context_clone = Arc::clone(&context);
            let emitter_clone = Arc::clone(&emitter);
            let workspace_clone = Arc::clone(&self.workspace);
            let project_graph_clone = Arc::clone(&self.project_graph);

            tokio::spawn(async move {
                while let Ok((mut action, permit)) = receiver.recv().await {
                    process_action(
                        &mut action,
                        Arc::clone(&context_clone),
                        Arc::clone(&emitter_clone),
                        Arc::clone(&workspace_clone),
                        Arc::clone(&project_graph_clone),
                    )
                    .await
                    .unwrap();

                    if action.has_failed() {
                        failed_count += 1;
                    } else if action.was_cached() {
                        cached_count += 1;
                    } else {
                        passed_count += 1;
                    }

                    // if self.bail && action.has_failed() || action.should_abort() {
                    //     abort = action.error.clone();
                    // }

                    // results.push(action);

                    drop(permit);
                }
            });
        }

        // Queue actions in topological order that need to be processed,
        // grouped into batches based on dependency requirements
        let total_actions_count = dep_graph.get_node_count();
        let batches = dep_graph.sort_batched_topological()?;
        let batches_count = batches.len();

        debug!(
            target: LOG_TARGET,
            "Running {} actions across {} batches", total_actions_count, batches_count
        );

        local_emitter
            .emit(Event::RunnerStarted {
                actions_count: total_actions_count,
            })
            .await?;

        for (b, batch) in batches.into_iter().enumerate() {
            let batch_index = b + 1;
            let batch_target_name = format!("{}:batch:{}", LOG_TARGET, batch_index);
            let actions_count = batch.len();

            // If a previous batch encountered an error that should abort,
            // handle it at the start of the next batch so that currently running
            // processes have time to finish.
            if let Some(abort_message) = abort {
                error!(
                    target: &batch_target_name,
                    "Encountered a critical error, aborting the action pipeline"
                );

                local_emitter
                    .emit(Event::RunnerAborted {
                        error: abort_message.clone(),
                    })
                    .await?;

                return Err(PipelineError::Aborted(abort_message));
            }

            trace!(
                target: &batch_target_name,
                "Running {} actions in batch {}",
                actions_count,
                batch_index
            );

            // We use a semaphore for ensuring that all actions within this batch
            // have completed processing before continuing to the next batch
            let semaphore = Arc::new(Semaphore::new(actions_count));

            for (i, node_index) in batch.into_iter().enumerate() {
                let action_index = i + 1;

                if let Some(node) = dep_graph.get_node_from_index(&node_index) {
                    let permit = semaphore.clone().acquire_owned().await.unwrap();
                    let mut action = Action::new(node.to_owned());

                    action.log_target = format!("{}:{}", batch_target_name, action_index);

                    let _ = sender.send((action, permit)).await;
                } else {
                    panic!("HOW?");
                }
            }

            // Wait for all actions in this batch to complete
            while semaphore.available_permits() != actions_count {
                continue;
            }

            semaphore.close();
        }

        let duration = start.elapsed();

        debug!(
            target: LOG_TARGET,
            "Finished running {} actions in {:?}", total_actions_count, &duration
        );

        local_emitter
            .emit(Event::RunnerFinished {
                duration: &duration,
                cached_count,
                failed_count,
                passed_count,
            })
            .await?;

        self.duration = Some(duration);
        // self.create_run_report(&results, context).await?;

        Ok(())
    }
}

async fn create_emitter(workspace: Arc<RwLock<Workspace>>) -> Emitter {
    let mut emitter = Emitter::new(Arc::clone(&workspace));

    {
        let local_workspace = workspace.read().await;

        // For security and privacy purposes, only send webhooks from a CI environment
        if is_ci() || is_test_env() {
            if let Some(webhook_url) = &local_workspace.config.notifier.webhook_url {
                emitter
                    .subscribers
                    .push(Arc::new(RwLock::new(WebhooksSubscriber::new(
                        webhook_url.to_owned(),
                    ))));
            }
        }

        if local_workspace.session.is_some() {
            emitter
                .subscribers
                .push(Arc::new(RwLock::new(MoonbaseCacheSubscriber::new())));
        }
    }

    // Must be last as its the final line of defense
    emitter
        .subscribers
        .push(Arc::new(RwLock::new(LocalCacheSubscriber::new())));

    emitter
}
