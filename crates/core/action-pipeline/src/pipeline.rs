use moon_action::{Action, ActionNode};
use moon_action_context::ActionContext;
use moon_dep_graph::DepGraph;
use moon_logger::{color, debug, error, trace};
use std::num::NonZeroUsize;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

const LOG_TARGET: &str = "moon:action-pipeline";

pub type ActionResults = Vec<Action>;

pub struct Pipeline {
    concurrency: usize,

    dep_graph: DepGraph,

    duration: Option<Duration>,
}

impl Pipeline {
    pub fn new(dep_graph: DepGraph) -> Self {
        let concurrency = thread::available_parallelism()
            .unwrap_or(NonZeroUsize::new(8).unwrap())
            .get();

        Pipeline {
            concurrency,
            dep_graph,
            duration: None,
        }
    }

    pub fn concurrency(&mut self, value: usize) -> &Self {
        self.concurrency = value;
        self
    }

    pub async fn run(&mut self, context: Option<ActionContext>) {
        let start = Instant::now();
        let context = context.unwrap_or_default();
        let mut results: ActionResults = vec![];

        // We use an async channel to coordinate actions (tasks) to process
        // across a bounded worker pool, as defined by the provided concurrency
        let (sender, receiver) = async_channel::unbounded::<(Action, OwnedSemaphorePermit)>();

        // Spawn worker threads that will process the action queue
        for _ in 0..self.concurrency {
            let receiver = receiver.clone();

            tokio::spawn(async move {
                while let Ok((action, permit)) = receiver.recv().await {
                    trace!(
                        target: &action.log_target,
                        "Running action {}",
                        color::muted_light(&action.label)
                    );

                    drop(permit);
                }
            });
        }

        // Queue actions in topological order that need to be processed,
        // grouped into batches based on dependency requirements
        let total_actions_count = self.dep_graph.get_node_count();
        let batches = self.dep_graph.sort_batched_topological().unwrap();
        let batches_count = batches.len();

        debug!(
            target: LOG_TARGET,
            "Running {} actions across {} batches", total_actions_count, batches_count
        );

        for (b, batch) in batches.into_iter().enumerate() {
            let batch_index = b + 1;
            let batch_target_name = format!("{}:batch:{}", LOG_TARGET, batch_index);
            let actions_count = batch.len();

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

                if let Some(node) = self.dep_graph.get_node_from_index(&node_index) {
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

        self.duration = Some(duration);
    }
}
