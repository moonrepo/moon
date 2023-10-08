use crate::errors::PipelineError;
use crate::estimator::Estimator;
use crate::processor::process_action;
use crate::run_report::RunReport;
use crate::subscribers::local_cache::LocalCacheSubscriber;
use crate::subscribers::moonbase::MoonbaseSubscriber;
use console::Term;
use moon_action::{Action, ActionNode, ActionStatus};
use moon_action_context::ActionContext;
use moon_action_graph::{ActionGraph, ActionGraphIter};
use moon_emitter::{Emitter, Event};
use moon_logger::{debug, error, trace};
use moon_notifier::WebhooksSubscriber;
use moon_project_graph::ProjectGraph;
use moon_terminal::{label_checkpoint, label_to_the_moon, Checkpoint, ExtendedTerm};
use moon_utils::{is_ci, is_test_env, time};
use moon_workspace::Workspace;
use starbase_styles::color;
use std::mem;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{RwLock, RwLockReadGuard, Semaphore};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

const LOG_TARGET: &str = "moon:action-pipeline";

pub type ActionResults = Vec<Action>;

pub struct Pipeline {
    bail: bool,

    concurrency: Option<usize>,

    duration: Option<Duration>,

    project_graph: Arc<RwLock<ProjectGraph>>,

    report_name: Option<String>,

    workspace: Arc<RwLock<Workspace>>,
}

impl Pipeline {
    pub fn new(workspace: Workspace, project_graph: ProjectGraph) -> Self {
        Pipeline {
            bail: false,
            concurrency: None,
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
        self.concurrency = Some(value);
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
        let start = Instant::now();
        let context = Arc::new(RwLock::new(context.unwrap_or_default()));
        let emitter = Arc::new(RwLock::new(
            create_emitter(Arc::clone(&self.workspace)).await,
        ));
        let workspace = Arc::clone(&self.workspace);
        let project_graph = Arc::clone(&self.project_graph);

        // Queue actions in topological order that need to be processed
        let total_actions_count = action_graph.get_node_count();
        let local_emitter = emitter.read().await;

        debug!(
            target: LOG_TARGET,
            "Running {} actions", total_actions_count
        );

        local_emitter
            .emit(Event::PipelineStarted {
                actions_count: total_actions_count,
                context: &*context.read().await,
            })
            .await?;

        // Launch a separate thread to listen for ctrl+c
        let cancel_token = CancellationToken::new();
        let ctrl_c_token = cancel_token.clone();

        tokio::spawn(async move {
            if tokio::signal::ctrl_c().await.is_ok() {
                ctrl_c_token.cancel();
            }
        });

        // This limits how many tasks can run in parallel
        let semaphore = Arc::new(Semaphore::new(
            self.concurrency.unwrap_or_else(num_cpus::get),
        ));

        let mut results: ActionResults = vec![];
        let mut action_handles = vec![];
        let mut persistent_nodes = vec![];
        let mut action_graph_iter = action_graph.try_iter()?;

        while action_graph_iter.has_pending() {
            let Some(node_index) = action_graph_iter.next() else {
                // Nothing to run since they're waiting on currently running
                // actions, so exhause the current list
                self.run_handles(
                    mem::take(&mut action_handles),
                    &mut action_graph_iter,
                    &mut results,
                    &local_emitter,
                )
                .await?;

                continue;
            };

            let Some(node) = action_graph.get_node_from_index(&node_index) else {
                continue;
            };

            // Run these later as a parallel batch
            if node.is_persistent() {
                trace!(
                    target: LOG_TARGET,
                    // index = node_index.index(),
                    "Marking action {} as persistent, will defer run",
                    node_index.index()
                );

                // Must mark as completed otherwise the loop hangs
                action_graph_iter.mark_completed(node_index.index());

                persistent_nodes.push(node_index);

                continue;
            }

            let context_clone = Arc::clone(&context);
            let emitter_clone = Arc::clone(&emitter);
            let workspace_clone = Arc::clone(&workspace);
            let project_graph_clone = Arc::clone(&project_graph);
            let cancel_token_clone = cancel_token.clone();
            let mut action = Action::new(node.to_owned());
            action.node_index = node_index.index();

            let Ok(permit) = semaphore.clone().acquire_owned().await else {
                continue; // Should error?
            };

            action_handles.push(tokio::spawn(async move {
                let result = tokio::select! {
                    biased;

                    _ = cancel_token_clone.cancelled() => {
                        Err(PipelineError::Aborted("Received ctrl + c, shutting down".into()).into())
                    }
                    res = process_action(
                        action,
                        context_clone,
                        emitter_clone,
                        workspace_clone,
                        project_graph_clone,
                    ) => res
                };

                drop(permit);

                result
            }));

            // Run this in isolation by exhausting the current list of handles
            if node.is_interactive() || semaphore.available_permits() == 0 {
                self.run_handles(
                    mem::take(&mut action_handles),
                    &mut action_graph_iter,
                    &mut results,
                    &local_emitter,
                )
                .await?;
            }
        }

        if !persistent_nodes.is_empty() {
            trace!(
                target: LOG_TARGET,
                "Running {} persistent actions",
                persistent_nodes.len(),
            );
        }

        for node_index in persistent_nodes {
            let Some(node) = action_graph.get_node_from_index(&node_index) else {
                continue;
            };

            let context_clone = Arc::clone(&context);
            let emitter_clone = Arc::clone(&emitter);
            let workspace_clone = Arc::clone(&workspace);
            let project_graph_clone = Arc::clone(&project_graph);
            let cancel_token_clone = cancel_token.clone();
            let mut action = Action::new(node.to_owned());
            action.node_index = node_index.index();

            let Ok(permit) = semaphore.clone().acquire_owned().await else {
                break; // Should error?
            };

            action_handles.push(tokio::spawn(async move {
                let result = tokio::select! {
                    biased;

                    _ = cancel_token_clone.cancelled() => {
                        Err(PipelineError::Aborted("Received ctrl + c, shutting down".into()).into())
                    }
                    res = process_action(
                        action,
                        context_clone,
                        emitter_clone,
                        workspace_clone,
                        project_graph_clone,
                    ) => res
                };

                drop(permit);

                result
            }));
        }

        // Run any remaining actions
        self.run_handles(
            action_handles,
            &mut action_graph_iter,
            &mut results,
            &local_emitter,
        )
        .await?;

        let duration = start.elapsed();
        let estimate = Estimator::calculate(&results, duration);
        let context = Arc::into_inner(context).unwrap().into_inner();
        let mut passed_count = 0;
        let mut cached_count = 0;
        let mut failed_count = 0;

        for result in &results {
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

        local_emitter
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
        self.create_run_report(&results, &context, estimate).await?;

        Ok(results)
    }

    async fn run_handles(
        &self,
        handles: Vec<JoinHandle<miette::Result<Action>>>,
        graph_iter: &mut ActionGraphIter<'_>,
        results: &mut ActionResults,
        emitter: &RwLockReadGuard<'_, Emitter>,
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
                        graph_iter.mark_completed(result.node_index);

                        show_abort_log = result.should_abort();

                        if self.bail && result.should_bail() || result.should_abort() {
                            abort_error = Some(result.get_error());
                        } else {
                            results.push(result);
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

    pub fn render_summary(&self, results: &ActionResults) -> miette::Result<()> {
        let term = Term::buffered_stdout();
        term.line("")?;

        let mut count = 0;

        for result in results {
            if !result.has_failed() {
                continue;
            }

            term.line(label_checkpoint(
                match &result.node {
                    Some(ActionNode::RunTask { target, .. }) => target.as_str(),
                    _ => &result.label,
                },
                Checkpoint::RunFailed,
            ))?;

            if let Some(attempts) = &result.attempts {
                if let Some(attempt) = attempts.iter().find(|a| a.has_failed()) {
                    let mut has_stdout = false;

                    if let Some(stdout) = &attempt.stdout {
                        if !stdout.is_empty() {
                            has_stdout = true;
                            term.line(stdout)?;
                        }
                    }

                    if let Some(stderr) = &attempt.stderr {
                        if has_stdout {
                            term.line("")?;
                        }

                        if !stderr.is_empty() {
                            term.line(stderr)?;
                        }
                    }
                }
            }

            term.line("")?;
            count += 1;
        }

        if count == 0 {
            term.line("No failed actions to summarize.")?;
        }

        term.line("")?;
        term.flush_lines()?;

        Ok(())
    }

    pub fn render_results(&self, results: &ActionResults) -> miette::Result<bool> {
        let term = Term::buffered_stdout();
        term.line("")?;

        let mut failed = false;

        for result in results {
            let status = match result.status {
                ActionStatus::Passed | ActionStatus::Cached | ActionStatus::CachedFromRemote => {
                    color::success("pass")
                }
                ActionStatus::Failed | ActionStatus::FailedAndAbort => {
                    if !result.allow_failure {
                        failed = true;
                    }

                    color::failure("fail")
                }
                ActionStatus::Invalid => color::invalid("warn"),
                ActionStatus::Skipped => color::muted_light("skip"),
                _ => color::muted_light("oops"),
            };

            let mut meta: Vec<String> = vec![];

            if matches!(
                result.status,
                ActionStatus::Cached | ActionStatus::CachedFromRemote
            ) {
                meta.push(String::from("cached"));
            } else if matches!(result.status, ActionStatus::Skipped) {
                meta.push(String::from("skipped"));
            } else if let Some(duration) = result.duration {
                meta.push(time::elapsed(duration));
            }

            term.line(format!(
                "{} {} {}",
                status,
                // color::create_style(&result.label).bold().to_string(),
                &result.label,
                color::muted(format!("({})", meta.join(", ")))
            ))?;
        }

        term.line("")?;
        term.flush_lines()?;

        Ok(failed)
    }

    pub fn render_stats(&self, results: &ActionResults, compact: bool) -> miette::Result<()> {
        let mut cached_count = 0;
        let mut pass_count = 0;
        let mut fail_count = 0;
        let mut invalid_count = 0;
        let mut skipped_count = 0;

        for result in results {
            if compact && !matches!(result.node.as_ref().unwrap(), ActionNode::RunTask { .. }) {
                continue;
            }

            match result.status {
                ActionStatus::Cached | ActionStatus::CachedFromRemote => {
                    cached_count += 1;
                    pass_count += 1;
                }
                ActionStatus::Passed => {
                    pass_count += 1;
                }
                ActionStatus::Failed | ActionStatus::FailedAndAbort => {
                    fail_count += 1;
                }
                ActionStatus::Invalid => {
                    invalid_count += 1;
                }
                ActionStatus::Skipped => {
                    skipped_count += 1;
                }
                _ => {}
            }
        }

        let mut counts_message = vec![];

        if pass_count > 0 {
            if cached_count > 0 {
                counts_message.push(color::success(format!(
                    "{pass_count} completed ({cached_count} cached)"
                )));
            } else {
                counts_message.push(color::success(format!("{pass_count} completed")));
            }
        }

        if fail_count > 0 {
            counts_message.push(color::failure(format!("{fail_count} failed")));
        }

        if invalid_count > 0 {
            counts_message.push(color::invalid(format!("{invalid_count} invalid")));
        }

        if skipped_count > 0 {
            counts_message.push(color::muted_light(format!("{skipped_count} skipped")));
        }

        let term = Term::buffered_stdout();
        term.line("")?;

        let counts_message = counts_message.join(&color::muted(", "));
        let mut elapsed_time = time::elapsed(self.duration.unwrap());

        if pass_count == cached_count && fail_count == 0 {
            elapsed_time = format!("{} {}", elapsed_time, label_to_the_moon());
        }

        if compact {
            term.render_entry("Tasks", &counts_message)?;
            term.render_entry(" Time", &elapsed_time)?;
        } else {
            term.render_entry("Actions", &counts_message)?;
            term.render_entry("   Time", &elapsed_time)?;
        }

        term.line("")?;
        term.flush_lines()?;

        Ok(())
    }

    async fn create_run_report(
        &self,
        actions: &ActionResults,
        context: &ActionContext,
        estimate: Estimator,
    ) -> miette::Result<()> {
        if let Some(name) = &self.report_name {
            let workspace = self.workspace.read().await;
            let duration = self.duration.unwrap();

            workspace
                .cache_engine
                .write(name, &RunReport::new(actions, context, duration, estimate))?;
        }

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
                .push(Arc::new(RwLock::new(MoonbaseSubscriber::new())));
        }
    }

    // Must be last as its the final line of defense
    emitter
        .subscribers
        .push(Arc::new(RwLock::new(LocalCacheSubscriber::new())));

    emitter
}
