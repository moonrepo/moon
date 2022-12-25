use moon_dep_graph::DepGraph;
use std::num::NonZeroUsize;
use std::thread;
use tokio::sync::mpsc;

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

    pub async fn run(&self) {
        let (mut sender, receiver) = mpsc::channel(100);

        // Spawn worker threads that will process the action queue
        for _ in 0..self.concurrency {
            tokio::spawn(async move {
                while let Some(action) = receiver.recv().await {
                    action.run();
                }
            });
        }

        // Spawn tasks for actions that need to be executed
    }
}
