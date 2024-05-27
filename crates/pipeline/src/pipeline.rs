use crate::context::Context;
use crate::job::JobResult;
use crate::pipeline_events::*;
use crate::step::*;
use starbase_events::Emitter;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::Semaphore;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::debug;

// TODO: run/ran events

pub struct Pipeline<T> {
    pub on_job_progress: Arc<Emitter<JobProgressEvent>>,
    pub on_job_state_change: Arc<Emitter<JobStateChangeEvent>>,

    bail: bool,
    concurrency: Option<usize>,
    steps: Vec<Box<dyn Step<T>>>,
}

impl<T: 'static> Pipeline<T> {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            on_job_progress: Arc::new(Emitter::new()),
            on_job_state_change: Arc::new(Emitter::new()),
            bail: false,
            concurrency: None,
            steps: vec![],
        }
    }

    pub fn bail_on_failure(&mut self) -> &mut Self {
        self.bail = true;
        self
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
        let total_steps = self.steps.len();
        let total_jobs: usize = self.steps.iter().map(|s| s.get_job_count()).sum();
        let mut complete_jobs = 0;
        let mut ran_jobs = 0;

        debug!(concurrency, total_steps, total_jobs, "Running pipeline");

        // This aggregates results from ran jobs
        let (sender, mut receiver) = mpsc::channel::<JobResult<T>>(total_jobs);

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
        let signal_handle = monitor_signals(context.cancel_token.clone());

        // Run our steps one-by-one
        let steps_handle = process_steps(self.steps, context.clone());

        // Wait for our results or for jobs to shutdown
        drop(sender);

        let mut results = vec![];

        while let Some(result) = receiver.recv().await {
            ran_jobs += 1;

            if result.state.is_complete() {
                complete_jobs += 1;
            }

            // TODO: allow failure logic
            if self.bail && result.state.has_failed() {
                context.abort();
            }

            results.push(result);

            if ran_jobs == total_jobs || context.is_aborted_or_cancelled() {
                break;
            }
        }

        debug!(
            complete_jobs,
            incomplete_jobs = total_jobs - complete_jobs,
            "Finished running pipeline"
        );

        // Clean up any open handles
        steps_handle.abort();
        signal_handle.abort();

        Ok(results)
    }
}

fn monitor_signals(cancel_token: CancellationToken) -> JoinHandle<()> {
    tokio::spawn(async move {
        debug!("Listening for ctrl+c signal");

        if tokio::signal::ctrl_c().await.is_ok() {
            debug!("Received ctrl+c signal, shutting down!");

            cancel_token.cancel();
        }
    })
}

fn process_steps<T: 'static>(steps: Vec<Box<dyn Step<T>>>, context: Context<T>) -> JoinHandle<()> {
    tokio::spawn(async move {
        for step in steps {
            let ctx = context.clone();
            step.run(ctx).await;
        }
    })
}
