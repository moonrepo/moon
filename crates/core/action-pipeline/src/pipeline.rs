use crate::errors::PipelineError;
use crate::estimator::Estimator;
use crate::processor::process_action;
use crate::run_report::RunReport;
use crate::subscribers::local_cache::LocalCacheSubscriber;
use crate::subscribers::moonbase::MoonbaseSubscriber;
use console::Term;
use moon_action::{Action, ActionStatus};
use moon_action_context::ActionContext;
use moon_dep_graph::DepGraph;
use moon_emitter::{Emitter, Event};
use moon_error::MoonError;
use moon_logger::{color, debug, error, trace};
use moon_notifier::WebhooksSubscriber;
use moon_project_graph::ProjectGraph;
use moon_terminal::{label_to_the_moon, replace_style_tokens, ExtendedTerm};
use moon_utils::{is_ci, is_test_env, time};
use moon_workspace::Workspace;
use rusty_pool::ThreadPool;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

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
        dep_graph: DepGraph,
        context: Option<ActionContext>,
    ) -> Result<ActionResults, PipelineError> {
        let start = Instant::now();
        let context = Arc::new(RwLock::new(context.unwrap_or_default()));
        let emitter = Arc::new(RwLock::new(
            create_emitter(Arc::clone(&self.workspace)).await,
        ));
        let workspace = Arc::clone(&self.workspace);
        let project_graph = Arc::clone(&self.project_graph);
        let mut results: ActionResults = vec![];
        let mut passed_count = 0;
        let mut cached_count = 0;
        let mut failed_count = 0;

        // Queue actions in topological order that need to be processed,
        // grouped into batches based on dependency requirements
        let total_actions_count = dep_graph.get_node_count();
        let batches = dep_graph.sort_batched_topological()?;
        let batches_count = batches.len();
        let local_emitter = emitter.read().await;

        debug!(
            target: LOG_TARGET,
            "Running {} actions across {} batches", total_actions_count, batches_count
        );

        local_emitter
            .emit(Event::PipelineStarted {
                actions_count: total_actions_count,
            })
            .await?;

        let pool = if let Some(concurrency) = &self.concurrency {
            ThreadPool::new(*concurrency, *concurrency, Duration::from_secs(30))
        } else {
            ThreadPool::default()
        };

        for (b, batch) in batches.into_iter().enumerate() {
            let batch_index = b + 1;
            let batch_target_name = format!("{LOG_TARGET}:batch:{batch_index}");
            let actions_count = batch.len();
            let mut action_handles = vec![];

            trace!(
                target: &batch_target_name,
                "Running {} actions in batch {}",
                actions_count,
                batch_index
            );

            for (i, node_index) in batch.into_iter().enumerate() {
                let action_index = i + 1;

                if let Some(node) = dep_graph.get_node_from_index(&node_index) {
                    let context_clone = Arc::clone(&context);
                    let emitter_clone = Arc::clone(&emitter);
                    let workspace_clone = Arc::clone(&workspace);
                    let project_graph_clone = Arc::clone(&project_graph);
                    let runtime = tokio::runtime::Handle::current();

                    let mut action = Action::new(node.to_owned());
                    action.log_target = format!("{batch_target_name}:{action_index}");

                    action_handles.push(pool.complete(async move {
                        runtime
                            .spawn(async move {
                                process_action(
                                    action,
                                    Arc::clone(&context_clone),
                                    Arc::clone(&emitter_clone),
                                    Arc::clone(&workspace_clone),
                                    Arc::clone(&project_graph_clone),
                                )
                                .await
                            })
                            .await
                    }));
                } else {
                    return Err(PipelineError::UnknownActionNode);
                }
            }

            // Wait for all actions in this batch to complete
            for handle in action_handles {
                match handle.try_await_complete() {
                    Ok(Ok(Ok(result))) => {
                        if result.has_failed() {
                            failed_count += 1;
                        } else if result.was_cached() {
                            cached_count += 1;
                        } else {
                            passed_count += 1;
                        }

                        if result.should_abort() {
                            error!(
                                target: &batch_target_name,
                                "Encountered a critical error, aborting the action pipeline"
                            );
                        }

                        if self.bail && result.has_failed() || result.should_abort() {
                            let abort_error =
                                result.error.unwrap_or_else(|| "Unknown error!".into());

                            local_emitter
                                .emit(Event::PipelineAborted {
                                    error: abort_error.clone(),
                                })
                                .await?;

                            return Err(PipelineError::Aborted(abort_error));
                        }

                        results.push(result);
                    }
                    Ok(Ok(Err(error))) => {
                        return Err(PipelineError::Aborted(error.to_string()));
                    }
                    _ => {
                        // What to do here?
                        return Err(PipelineError::Aborted("Unknown error!".to_owned()));
                    }
                };
            }
        }

        let duration = start.elapsed();
        let estimate = Estimator::calculate(&results, duration);

        debug!(
            target: LOG_TARGET,
            "Finished running {} actions in {:?}", total_actions_count, &duration
        );

        local_emitter
            .emit(Event::PipelineFinished {
                baseline_duration: &estimate.duration,
                cached_count,
                duration: &duration,
                estimated_savings: estimate.gain.as_ref(),
                failed_count,
                passed_count,
            })
            .await?;

        self.duration = Some(duration);
        self.create_run_report(&results, context, estimate).await?;

        Ok(results)
    }

    pub fn render_results(&self, results: &ActionResults) -> Result<bool, MoonError> {
        let term = Term::buffered_stdout();
        term.write_line("")?;

        let mut failed = false;

        for result in results {
            let status = match result.status {
                ActionStatus::Passed
                | ActionStatus::Cached
                | ActionStatus::CachedFromRemote
                | ActionStatus::Skipped => color::success("pass"),
                ActionStatus::Failed | ActionStatus::FailedAndAbort => {
                    failed = true;
                    color::failure("fail")
                }
                ActionStatus::Invalid => color::invalid("warn"),
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

            term.write_line(&format!(
                "{} {} {}",
                status,
                color::style(&result.label).bold(),
                color::muted(format!("({})", meta.join(", ")))
            ))?;

            if let Some(error) = &result.error {
                term.write_line(&format!(
                    "     {}",
                    color::muted_light(replace_style_tokens(error))
                ))?;
            }
        }

        term.write_line("")?;
        term.flush()?;

        Ok(failed)
    }

    pub fn render_stats(&self, results: &ActionResults, compact: bool) -> Result<(), MoonError> {
        let mut cached_count = 0;
        let mut pass_count = 0;
        let mut fail_count = 0;
        let mut invalid_count = 0;

        for result in results {
            if compact && !result.label.contains("RunTarget") {
                continue;
            }

            match result.status {
                ActionStatus::Cached | ActionStatus::CachedFromRemote => {
                    cached_count += 1;
                    pass_count += 1;
                }
                ActionStatus::Passed | ActionStatus::Skipped => {
                    pass_count += 1;
                }
                ActionStatus::Failed | ActionStatus::FailedAndAbort => {
                    fail_count += 1;
                }
                ActionStatus::Invalid => {
                    invalid_count += 1;
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

        let term = Term::buffered_stdout();
        term.write_line("")?;

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

        term.write_line("")?;
        term.flush()?;

        Ok(())
    }

    async fn create_run_report(
        &self,
        actions: &ActionResults,
        context: Arc<RwLock<ActionContext>>,
        estimate: Estimator,
    ) -> Result<(), PipelineError> {
        if let Some(name) = &self.report_name {
            let workspace = self.workspace.read().await;
            let duration = self.duration.unwrap();
            let context = context.read().await;

            workspace
                .cache
                .create_json_report(name, RunReport::new(actions, &context, duration, estimate))?;
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
