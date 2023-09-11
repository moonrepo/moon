use crate::pipe::*;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;
use tracing::debug;
use tracing::warn;

#[derive(Default)]
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

        debug!(concurrency, "Running pipeline");

        // This aggregates results from ran jobs
        let (sender, mut receiver) = mpsc::channel::<u8>(10);

        // This limits how many jobs can run in parallel
        let semaphore = Arc::new(Semaphore::new(concurrency));

        // This determines whether to cancel/shutdown running tasks
        let cancel_token = CancellationToken::new();

        // Monitor signals and ctrl+c
        monitor_signals(cancel_token.clone());

        // Run our pipes (jobs) one-by-one
        let total_pipes = self.pipes.len();
        let mut ran_pipes = 0;

        let handle = PipeHandle {
            cancel_token: cancel_token.clone(),
            semaphore: semaphore.clone(),
            result_sender: sender.clone(),
        };

        for pipe in self.pipes {
            let pipe_handle = pipe.run(handle.clone()).await;

            // Wait for the handle to complete, as pipes are ran serially
            if pipe_handle.await.is_err() {
                cancel_token.cancel();
            }
        }

        // Wait for our results or for jobs to shutdown
        drop(sender);

        while let Some(result) = receiver.recv().await {
            ran_pipes += 1;
            println!("got = {}", result);

            if ran_pipes == total_pipes || cancel_token.is_cancelled() {
                break;
            }
        }
    }
}

fn monitor_signals(cancel_token: CancellationToken) {
    tokio::spawn(async move {
        debug!("Listening for ctrl+c signal");

        if tokio::signal::ctrl_c().await.is_ok() {
            warn!("Received ctrl+c signal, shutting down!");

            cancel_token.cancel();
        }
    });
}
