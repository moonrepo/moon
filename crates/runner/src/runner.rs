use crate::dep_graph::DepGraph;
use crate::emitter::{Event, RunnerEmitter};
use crate::errors::{DepGraphError, RunnerError};
use crate::node::ActionNode;
use console::Term;
use moon_action::{Action, ActionContext, ActionStatus};
use moon_cache::RunReport;
use moon_error::MoonError;
use moon_logger::{color, debug, error, trace};
use moon_terminal::{label_to_the_moon, replace_style_tokens, ExtendedTerm};
use moon_utils::time;
use moon_workspace::Workspace;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tokio::task;

const LOG_TARGET: &str = "moon:runner";

pub type ActionResults = Vec<Action>;

async fn run_action(
    node: &ActionNode,
    action: &mut Action,
    context: &ActionContext,
    workspace: Arc<RwLock<Workspace>>,
    emitter: Arc<RwLock<RunnerEmitter>>,
) -> Result<(), RunnerError> {
    match node.run(action, context, workspace, emitter).await {
        Ok(status) => {
            action.done(status);
        }
        Err(error) => {
            action.fail(error.to_string());

            // If these fail, we should abort instead of trying to continue
            if matches!(node, ActionNode::SetupTool(_))
                || matches!(node, ActionNode::InstallDeps(_))
            {
                action.abort();
            }
        }
    }

    Ok(())
}

pub struct ActionRunner {
    bail: bool,

    cached_count: usize,

    duration: Option<Duration>,

    failed_count: usize,

    passed_count: usize,

    report_name: Option<String>,

    workspace: Arc<RwLock<Workspace>>,
}

impl ActionRunner {
    pub fn new(workspace: Workspace) -> Self {
        debug!(target: LOG_TARGET, "Creating action runner");

        ActionRunner {
            bail: false,
            cached_count: 0,
            duration: None,
            failed_count: 0,
            passed_count: 0,
            report_name: None,
            workspace: Arc::new(RwLock::new(workspace)),
        }
    }

    pub fn bail_on_error(&mut self) -> &mut Self {
        self.bail = true;
        self
    }

    pub fn generate_report(&mut self, name: &str) -> &mut Self {
        self.report_name = Some(name.to_owned());
        self
    }

    pub fn get_duration(&self) -> Duration {
        self.duration
            .expect("Cannot get duration, action runner not ran!")
    }

    pub fn has_failed(&self) -> bool {
        self.failed_count > 0
    }

    pub async fn run(
        &mut self,
        graph: DepGraph,
        context: Option<ActionContext>,
    ) -> Result<ActionResults, RunnerError> {
        let start = Instant::now();
        let node_count = graph.graph.node_count();
        let batches = graph.sort_batched_topological()?;
        let batches_count = batches.len();
        let graph = Arc::new(RwLock::new(graph));
        let context = Arc::new(context.unwrap_or_default());
        let emitter = Arc::new(RwLock::new(RunnerEmitter::new(Arc::clone(&self.workspace))));
        let local_emitter = emitter.read().await;

        debug!(
            target: LOG_TARGET,
            "Running {} actions across {} batches", node_count, batches_count
        );

        local_emitter
            .emit(Event::RunStarted {
                actions_count: node_count,
            })
            .await?;

        let mut results: ActionResults = vec![];

        for (b, batch) in batches.into_iter().enumerate() {
            let batch_count = b + 1;
            let batch_target_name = format!("{}:batch:{}", LOG_TARGET, batch_count);
            let actions_count = batch.len();

            trace!(
                target: &batch_target_name,
                "Running {} actions",
                actions_count
            );

            let mut action_handles = vec![];

            for (i, node_index) in batch.into_iter().enumerate() {
                let action_count = i + 1;
                let graph_clone = Arc::clone(&graph);
                let context_clone = Arc::clone(&context);
                let workspace_clone = Arc::clone(&self.workspace);
                let emitter_clone = Arc::clone(&emitter);

                action_handles.push(task::spawn(async move {
                    let mut action = Action::new(node_index.index(), None);
                    let own_graph = graph_clone.read().await;
                    let own_emitter = emitter_clone.read().await;

                    if let Some(node) = own_graph.get_node_from_index(&node_index) {
                        action.label = Some(node.label());

                        own_emitter
                            .emit(Event::ActionStarted {
                                action: &action,
                                node,
                            })
                            .await?;

                        let log_target_name =
                            format!("{}:batch:{}:{}", LOG_TARGET, batch_count, action_count);
                        let log_action_label = color::muted_light(&node.label());

                        trace!(
                            target: &log_target_name,
                            "Running action {}",
                            log_action_label
                        );

                        run_action(
                            node,
                            &mut action,
                            &context_clone,
                            workspace_clone,
                            Arc::clone(&emitter_clone),
                        )
                        .await?;

                        if action.has_failed() {
                            trace!(
                                target: &log_target_name,
                                "Failed to run action {} in {:?}",
                                log_action_label,
                                action.duration.unwrap()
                            );
                        } else {
                            trace!(
                                target: &log_target_name,
                                "Ran action {} in {:?}",
                                log_action_label,
                                action.duration.unwrap()
                            );
                        }

                        own_emitter
                            .emit(Event::ActionFinished {
                                action: &action,
                                node,
                            })
                            .await?;
                    } else {
                        action.status = ActionStatus::Invalid;

                        return Err(RunnerError::DepGraph(DepGraphError::UnknownNode(
                            node_index.index(),
                        )));
                    }

                    Ok(action)
                }));
            }

            // Wait for all actions in this batch to complete,
            // while also handling and propagating errors
            for handle in action_handles {
                match handle.await {
                    Ok(Ok(result)) => {
                        if result.should_abort() {
                            error!(
                                target: &batch_target_name,
                                "Encountered a critical error, aborting the action runner"
                            );
                        }

                        if result.has_failed() {
                            self.failed_count += 1;
                        } else if result.was_cached() {
                            self.cached_count += 1;
                        } else {
                            self.passed_count += 1;
                        }

                        if self.bail && result.has_failed() || result.should_abort() {
                            local_emitter.emit(Event::RunAborted).await?;

                            return Err(RunnerError::Failure(result.error.unwrap()));
                        }

                        results.push(result);
                    }
                    Ok(Err(e)) => {
                        self.failed_count += 1;
                        local_emitter.emit(Event::RunAborted).await?;

                        return Err(e);
                    }
                    Err(e) => {
                        self.failed_count += 1;
                        local_emitter.emit(Event::RunAborted).await?;

                        return Err(RunnerError::Failure(e.to_string()));
                    }
                }
            }
        }

        let duration = start.elapsed();

        debug!(
            target: LOG_TARGET,
            "Finished running {} actions in {:?}", node_count, &duration
        );

        local_emitter
            .emit(Event::RunFinished {
                duration: &duration,
                cached_count: self.cached_count,
                failed_count: self.failed_count,
                passed_count: self.passed_count,
            })
            .await?;

        self.duration = Some(duration);
        self.create_run_report(&results, &context).await?;

        Ok(results)
    }

    pub fn render_results(&self, results: &ActionResults) -> Result<(), MoonError> {
        let term = Term::buffered_stdout();
        term.write_line("")?;

        for result in results {
            let status = match result.status {
                ActionStatus::Passed | ActionStatus::Cached | ActionStatus::Skipped => {
                    color::success("pass")
                }
                ActionStatus::Failed | ActionStatus::FailedAndAbort => color::failure("fail"),
                ActionStatus::Invalid => color::invalid("warn"),
                _ => color::muted_light("oops"),
            };

            let mut meta: Vec<String> = vec![];

            if matches!(result.status, ActionStatus::Cached) {
                meta.push(String::from("cached"));
            } else if matches!(result.status, ActionStatus::Skipped) {
                meta.push(String::from("skipped"));
            } else if let Some(duration) = result.duration {
                meta.push(time::elapsed(duration));
            }

            term.write_line(&format!(
                "{} {} {}",
                status,
                color::style(result.label.as_ref().unwrap()).bold(),
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

        Ok(())
    }

    pub fn render_stats(&self, results: &ActionResults, compact: bool) -> Result<(), MoonError> {
        let mut cached_count = 0;
        let mut pass_count = 0;
        let mut fail_count = 0;
        let mut invalid_count = 0;

        for result in results {
            if let Some(label) = &result.label {
                if compact && !label.contains("RunTarget") {
                    continue;
                }
            }

            match result.status {
                ActionStatus::Cached => {
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
                    "{} completed ({} cached)",
                    pass_count, cached_count
                )));
            } else {
                counts_message.push(color::success(format!("{} completed", pass_count)));
            }
        }

        if fail_count > 0 {
            counts_message.push(color::failure(format!("{} failed", fail_count)));
        }

        if invalid_count > 0 {
            counts_message.push(color::invalid(format!("{} invalid", invalid_count)));
        }

        let term = Term::buffered_stdout();
        term.write_line("")?;

        let counts_message = counts_message.join(&color::muted(", "));
        let mut elapsed_time = time::elapsed(self.get_duration());

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
        context: &ActionContext,
    ) -> Result<(), RunnerError> {
        if let Some(name) = &self.report_name {
            let workspace = self.workspace.read().await;
            let duration = self.duration.unwrap();

            workspace
                .cache
                .create_json_report(name, RunReport::new(actions, context, duration))
                .await?;
        }

        Ok(())
    }
}
