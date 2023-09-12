use crate::context::Context;
use crate::step::*;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;
use tracing::debug;
use tracing::warn;

pub struct Pipeline<T> {
    concurrency: Option<usize>,
    steps: Vec<Box<dyn Step<T>>>,
}

impl<T> Pipeline<T> {
    pub fn new() -> Self {
        Self {
            concurrency: None,
            steps: vec![],
        }
    }

    pub fn concurrency(&mut self, value: usize) -> &mut Self {
        self.concurrency = Some(value);
        self
    }

    pub fn add_step(&mut self, step: impl Step<T> + 'static) -> &mut Self {
        self.steps.push(Box::new(step));
        self
    }

    pub async fn run(self) -> miette::Result<Vec<Option<T>>> {
        let concurrency = self.concurrency.unwrap_or_else(num_cpus::get);

        debug!(concurrency, "Running pipeline");

        // This aggregates results from ran jobs
        let (sender, mut receiver) = mpsc::channel::<Option<T>>(10);

        let context = Context {
            cancel_token: CancellationToken::new(),
            semaphore: Arc::new(Semaphore::new(concurrency)),
            result_sender: sender.clone(),
        };

        // Monitor signals and ctrl+c
        monitor_signals(context.cancel_token.clone());

        // Run our pipes (jobs) one-by-one
        let total_steps = self.steps.len();
        let mut complete_steps = 0;

        for step in self.steps {
            let handle = step.run(context.clone()).await;

            // Wait for the handle to complete, as steps are ran serially
            if handle.await.is_err() {
                context.cancel_token.cancel();
            }
        }

        // Wait for our results or for jobs to shutdown
        drop(sender);

        let mut results = vec![];

        while let Some(result) = receiver.recv().await {
            complete_steps += 1;
            results.push(result);

            if complete_steps == total_steps || context.cancel_token.is_cancelled() {
                break;
            }
        }

        Ok(results)
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
