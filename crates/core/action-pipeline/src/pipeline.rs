use moon_action::{Action, ActionNode};
use moon_action_context::ActionContext;
use moon_dep_graph::DepGraph;
use moon_logger::{color, debug, error, trace};
use std::num::NonZeroUsize;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tokio::sync::RwLock;

const LOG_TARGET: &str = "moon:action-pipeline";

pub type ActionResults = Vec<Action>;

pub struct Pipeline {
    concurrency: usize,
    dep_graph: DepGraph,
}

impl Pipeline {
    pub fn new(dep_graph: DepGraph) -> Self {
        let concurrency = thread::available_parallelism()
            .unwrap_or(NonZeroUsize::new(8).unwrap())
            .get();

        Pipeline {
            concurrency,
            dep_graph,
        }
    }

    pub fn concurrency(&mut self, value: usize) -> &Self {
        self.concurrency = value;
        self
    }

    pub async fn run(&self, context: Option<ActionContext>) {
        let (sender, receiver) = async_channel::unbounded();
        let mut results: ActionResults = vec![];

        // Spawn worker threads that will process the action queue
        for _ in 0..self.concurrency {
            let receiver = receiver.clone();

            tokio::spawn(async move {
                while let Ok(action) = receiver.recv().await {
                    dbg!("RECEIVED", &action);
                }
            });
        }

        // Spawn tasks for actions that need to be executed
        let start = Instant::now();
        let node_count = self.dep_graph.get_node_count();
        let batches = self.dep_graph.sort_batched_topological().unwrap();
        let batches_count = batches.len();
        let context = context.unwrap_or_default();

        debug!(
            target: LOG_TARGET,
            "Running {} actions across {} batches", node_count, batches_count
        );

        for (b, batch) in batches.into_iter().enumerate() {
            let batch_count = b + 1;
            let batch_target_name = format!("{}:batch:{}", LOG_TARGET, batch_count);
            let actions_count = batch.len();
            let mut action_handles = vec![];

            trace!(
                target: &batch_target_name,
                "Running {} actions",
                actions_count
            );

            for (i, node_index) in batch.into_iter().enumerate() {
                let action_count = i + 1;

                if let Some(node) = self.dep_graph.get_node_from_index(&node_index) {
                    let sender = sender.clone();
                    let log_target_name =
                        format!("{}:batch:{}:{}", LOG_TARGET, batch_count, action_count);
                    let log_action_label = color::muted_light(node.label());

                    trace!(
                        target: &log_target_name,
                        "Running action {}",
                        log_action_label
                    );

                    action_handles.push(tokio::spawn(async move {
                        let mut action = Action::new(node_index.index(), None);

                        sender.send(action).await.unwrap();

                        // action
                    }));
                } else {
                    panic!("HOW?");
                }
            }

            // Wait for all actions in this batch to complete,
            // while also handling and propagating errors
            for handle in action_handles {
                handle.await.unwrap();
            }
        }

        let duration = start.elapsed();

        debug!(
            target: LOG_TARGET,
            "Finished running {} actions in {:?}", node_count, &duration
        );
    }
}
