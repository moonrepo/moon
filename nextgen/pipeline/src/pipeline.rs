use crate::context::Context;
use crate::job::JobResult;
use crate::pipeline_events::*;
use crate::step::*;
use starbase_events::Emitter;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;
use tracing::debug;
use tracing::warn;

// TODO: bail on failure, run/ran events

pub struct Pipeline<T> {
    pub on_job_progress: Arc<Emitter<JobProgressEvent>>,
    pub on_job_state_change: Arc<Emitter<JobStateChangeEvent>>,

    concurrency: Option<usize>,
    steps: Vec<Box<dyn Step<T>>>,
}

impl<T> Pipeline<T> {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            on_job_progress: Arc::new(Emitter::new()),
            on_job_state_change: Arc::new(Emitter::new()),
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

    pub async fn run(self) -> miette::Result<Vec<JobResult<T>>> {
        self.run_with_context(|_| {}).await
    }

    pub async fn run_with_context(
        self,
        on_run: impl FnOnce(Context<T>),
    ) -> miette::Result<Vec<JobResult<T>>> {
        let concurrency = self.concurrency.unwrap_or_else(num_cpus::get);

        debug!(concurrency, "Running pipeline");

        // This aggregates results from ran jobs
        let (sender, mut receiver) = mpsc::channel::<JobResult<T>>(10);

        let context = Context {
            abort_token: CancellationToken::new(),
            cancel_token: CancellationToken::new(),
            semaphore: Arc::new(Semaphore::new(concurrency)),
            result_sender: sender.clone(),
            on_job_progress: Arc::clone(&self.on_job_progress),
            on_job_state_change: Arc::clone(&self.on_job_state_change),
        };

        on_run(context.clone());

        // Monitor signals and ctrl+c
        monitor_signals(context.cancel_token.clone());

        // Run our pipes (jobs) one-by-one
        let total_steps = self.steps.len();
        let mut complete_steps = 0;

        for step in self.steps {
            // Wait for the handle to complete, as steps are ran serially
            step.run(context.clone()).await;
        }

        // Wait for our results or for jobs to shutdown
        drop(sender);

        let mut results = vec![];

        while let Some(result) = receiver.recv().await {
            complete_steps += 1;
            results.push(result);

            if complete_steps == total_steps
                || context.abort_token.is_cancelled()
                || context.cancel_token.is_cancelled()
            {
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
