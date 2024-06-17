use crate::errors::PipelineError;
use crate::estimator::Estimator;
use crate::processor::process_action;
use crate::run_report::RunReport;
use crate::subscribers::local_cache::LocalCacheSubscriber;
use crate::subscribers::moonbase::MoonbaseSubscriber;
use moon_action::Action;
use moon_action_context::ActionContext;
use moon_action_graph::ActionGraph;
use moon_api::Moonbase;
use moon_app_context::AppContext;
use moon_console::PipelineReportItem;
use moon_emitter::{Emitter, Event};
use moon_logger::{debug, error, trace, warn};
use moon_notifier::WebhooksSubscriber;
use moon_project_graph::ProjectGraph;
use moon_utils::{is_ci, is_test_env};
use std::mem;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Semaphore;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

const LOG_TARGET: &str = "moon:action-pipeline";

pub type ActionResults = Vec<Action>;

pub struct Pipeline {
    aborted: bool,

    bail: bool,

    concurrency: Option<usize>,

    duration: Option<Duration>,

    project_graph: Arc<ProjectGraph>,

    report_name: Option<String>,

    results: Vec<Action>,

    summarize: bool,

    app_context: Arc<AppContext>,
}

impl Pipeline {
    pub fn new(app_context: Arc<AppContext>, project_graph: Arc<ProjectGraph>) -> Self {
        Pipeline {
            aborted: false,
            bail: false,
            concurrency: None,
            duration: None,
            project_graph,
            report_name: None,
            results: vec![],
            summarize: false,
            app_context,
        }
    }

    pub fn bail_on_error(&mut self) -> &mut Self {
        self.bail = true;
        self
    }

    pub fn concurrency(&mut self, value: usize) -> &Self {
        self.concurrency = Some(value);
        self
    }

    pub fn summarize(&mut self, value: bool) -> &mut Self {
        self.summarize = value;
        self
    }

    pub fn generate_report(&mut self, name: &str) -> &mut Self {
        self.report_name = Some(name.to_owned());
        self
    }

    pub async fn run(
        &mut self,
        action_graph: ActionGraph,
        context: Option<ActionContext>,
    ) -> miette::Result<ActionResults> {
        let result = self.run_internal(action_graph, context).await;

        let actions = mem::take(&mut self.results);

        let item = PipelineReportItem {
            duration: self.duration,
            summarize: self.summarize,
        };

        match result {
            Ok(_) => {
                self.app_context
                    .console
                    .reporter
                    .on_pipeline_completed(&actions, &item, None)?;

                Ok(actions)
            }
            Err(error) => {
                if self.aborted {
                    self.app_context.console.reporter.on_pipeline_aborted(
                        &actions,
                        &item,
                        Some(&error),
                    )?;
                } else {
                    self.app_context.console.reporter.on_pipeline_completed(
                        &actions,
                        &item,
                        Some(&error),
                    )?;
                }

                Err(error)
            }
        }
    }

    pub async fn run_internal(
        &mut self,
        action_graph: ActionGraph,
        context: Option<ActionContext>,
    ) -> miette::Result<()> {
        let start = Instant::now();
        let context = Arc::new(context.unwrap_or_default());
        let app_context = Arc::clone(&self.app_context);
        let emitter = Arc::new(create_emitter(Arc::clone(&self.app_context)).await);
        let project_graph = Arc::clone(&self.project_graph);

        // Queue actions in topological order that need to be processed
        let total_actions_count = action_graph.get_node_count();

        debug!(
            target: LOG_TARGET,
            "Running {} actions", total_actions_count
        );

        let mut action_handles = vec![];
        // let mut persistent_nodes = vec![];
        let mut action_graph_iter = action_graph.try_iter()?;

        action_graph_iter.monitor_completed();

        // while action_graph_iter.has_pending() {
        //     let context_clone = Arc::clone(&context);
        //     let emitter_clone = Arc::clone(&emitter);
        //     let workspace_clone = Arc::clone(&workspace);
        //     let project_graph_clone = Arc::clone(&project_graph);
        //     let app_context_clone = Arc::clone(&app_context);
        //     let cancel_token_clone = cancel_token.clone();
        //     let sender = action_graph_iter.sender.clone();
        //     let mut action = Action::new(node.to_owned());
        //     action.node_index = node_index.index();

        //     let Ok(permit) = semaphore.clone().acquire_owned().await else {
        //         continue; // Should error?
        //     };

        //     action_handles.push(tokio::spawn(async move {
        //         let interactive = action.node.is_interactive();
        //         let result = tokio::select! {
        //             biased;

        //             _ = cancel_token_clone.cancelled(), if !interactive => {
        //                 Err(PipelineError::Aborted("Received ctrl + c, shutting down".into()).into())
        //             }
        //             res = process_action(
        //                 action,
        //                 context_clone,
        //                 app_context_clone,
        //                 emitter_clone,
        //                 workspace_clone,
        //                 project_graph_clone,
        //             ) => res
        //         };

        //         if let Ok(action) = &result {
        //             let _ = sender.send(action.node_index);
        //         }

        //         drop(permit);

        //         result
        //     }));

        // }

        // if !persistent_nodes.is_empty() {
        //     trace!(
        //         target: LOG_TARGET,
        //         "Running {} persistent actions",
        //         persistent_nodes.len(),
        //     );
        // }

        // for node_index in persistent_nodes {
        //     let Some(node) = action_graph.get_node_from_index(&node_index) else {
        //         warn!(
        //             target: LOG_TARGET,
        //             "Received a graph index {} with no associated node, unable to process",
        //             node_index.index()
        //         );

        //         continue;
        //     };

        //     let context_clone = Arc::clone(&context);
        //     let emitter_clone = Arc::clone(&emitter);
        //     let workspace_clone = Arc::clone(&workspace);
        //     let project_graph_clone = Arc::clone(&project_graph);
        //     let app_context_clone = Arc::clone(&app_context);
        //     let cancel_token_clone = cancel_token.clone();
        //     let sender = action_graph_iter.sender.clone();
        //     let mut action = Action::new(node.to_owned());
        //     action.node_index = node_index.index();

        //     action_handles.push(tokio::spawn(async move {
        //         let interactive = action.node.is_interactive();
        //         let result = tokio::select! {
        //             biased;

        //             _ = cancel_token_clone.cancelled(), if !interactive => {
        //                 Err(PipelineError::Aborted("Received ctrl + c, shutting down".into()).into())
        //             }
        //             res = process_action(
        //                 action,
        //                 context_clone,
        //                 app_context_clone,
        //                 emitter_clone,
        //                 workspace_clone,
        //                 project_graph_clone,
        //             ) => res
        //         };

        //         if let Ok(action) = &result {
        //             let _ = sender.send(action.node_index);
        //         }

        //         result
        //     }));
        // }

        // Run any remaining actions
        self.run_handles(action_handles, &emitter).await?;

        let duration = start.elapsed();
        let estimate = Estimator::calculate(&self.results, duration);
        let context = Arc::into_inner(context).unwrap();
        let mut passed_count = 0;
        let mut cached_count = 0;
        let mut failed_count = 0;

        for result in &self.results {
            if result.has_failed() {
                failed_count += 1;
            } else if result.was_cached() {
                cached_count += 1;
            } else {
                passed_count += 1;
            }
        }

        debug!(
            target: LOG_TARGET,
            "Finished running {} actions in {:?}", total_actions_count, &duration
        );

        emitter
            .emit(Event::PipelineFinished {
                baseline_duration: &estimate.duration,
                cached_count,
                context: &context,
                duration: &duration,
                estimated_savings: estimate.gain.as_ref(),
                failed_count,
                passed_count,
            })
            .await?;

        self.duration = Some(duration);
        self.create_run_report(&self.results, &context, estimate)
            .await?;

        Ok(())
    }

    async fn run_handles(
        &mut self,
        handles: Vec<JoinHandle<miette::Result<Action>>>,
        emitter: &Emitter,
    ) -> miette::Result<()> {
        let mut abort_error: Option<miette::Report> = None;
        let mut show_abort_log = false;

        for handle in handles {
            if abort_error.is_some() {
                if !handle.is_finished() {
                    handle.abort();
                }
            } else {
                match handle.await {
                    Ok(Ok(mut result)) => {
                        show_abort_log = result.should_abort();

                        if self.bail && result.should_bail() || result.should_abort() {
                            abort_error = Some(result.get_error());
                        } else {
                            self.results.push(result);
                        }
                    }
                    Ok(Err(error)) => {
                        abort_error = Some(error);
                    }
                    _ => {
                        abort_error = Some(PipelineError::Aborted("Unknown error!".into()).into());
                    }
                };
            }
        }

        if let Some(abort_error) = abort_error {
            self.aborted = true;

            if show_abort_log {
                error!("Encountered a critical error, aborting the action pipeline");
            }

            emitter
                .emit(Event::PipelineAborted {
                    error: abort_error.to_string(),
                })
                .await?;

            return Err(abort_error);
        }

        Ok(())
    }

    async fn create_run_report(
        &self,
        actions: &ActionResults,
        context: &ActionContext,
        estimate: Estimator,
    ) -> miette::Result<()> {
        if let Some(name) = &self.report_name {
            let duration = self.duration.unwrap();

            self.app_context
                .cache_engine
                .write(name, &RunReport::new(actions, context, duration, estimate))?;
        }

        Ok(())
    }
}

async fn create_emitter(app_context: Arc<AppContext>) -> Emitter {
    let emitter = Emitter::new(Arc::clone(&app_context));

    // For security and privacy purposes, only send webhooks from a CI environment
    if is_ci() || is_test_env() {
        if let Some(webhook_url) = &app_context.workspace_config.notifier.webhook_url {
            emitter
                .subscribe(WebhooksSubscriber::new(webhook_url.to_owned()))
                .await;
        }
    }

    if Moonbase::session().is_some() {
        emitter.subscribe(MoonbaseSubscriber::new()).await;
    }

    // Must be last as its the final line of defense
    emitter.subscribe(LocalCacheSubscriber::new()).await;

    emitter
}
