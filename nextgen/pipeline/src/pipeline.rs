use crate::pipe::*;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::Semaphore;

pub struct Pipeline {
    concurrency: Option<usize>,
    pipes: Vec<Box<dyn Pipe>>,
}

impl Pipeline {
    pub fn concurrency(&mut self, value: usize) -> &mut Self {
        self.concurrency = Some(value);
        self
    }

    pub fn pipe(&mut self, pipe: impl Pipe + 'static) -> &mut Self {
        self.pipes.push(Box::new(pipe));
        self
    }

    pub async fn run(self) {
        let concurrency = self.concurrency.unwrap_or_else(num_cpus::get);

        // This aggregates results from ran jobs
        let (tx, mut rx) = mpsc::channel::<u8>(concurrency * 10);

        // This limits how many jobs can run in parallel
        let semaphore = Arc::new(Semaphore::new(concurrency));

        for pipe in self.pipes {
            PipeHandle::new(pipe, Arc::clone(&semaphore), tx.clone())
                .run()
                .await;

            // to do: abort/cancel
        }

        while let Some(result) = rx.recv().await {
            println!("got = {}", result);
        }
    }
}
