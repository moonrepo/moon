use crate::actions;
use crate::errors::RunnerError;
use crate::run_report::RunReport;
use crate::subscribers::local_cache::LocalCacheSubscriber;
use crate::subscribers::moonbase_cache::MoonbaseCacheSubscriber;
use console::Term;
use moon_action::{Action, ActionNode, ActionStatus};
use moon_dep_graph::{DepGraph, DepGraphError};
use moon_emitter::{Emitter, Event};
use moon_error::MoonError;
use moon_logger::{color, debug, error, trace};
use moon_node_platform::actions as node_actions;
use moon_notifier::WebhooksSubscriber;
use moon_platform_runtime::Runtime;
use moon_project_graph::ProjectGraph;
use moon_runner_context::RunnerContext;
use moon_task::Target;
use moon_terminal::{label_to_the_moon, replace_style_tokens, ExtendedTerm};
use moon_utils::{is_ci, is_test_env, time};
use moon_workspace::Workspace;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tokio::task;

const LOG_TARGET: &str = "moon:runner";

pub type ActionResults = Vec<Action>;

pub struct Runner {
    bail: bool,

    cached_count: usize,

    duration: Option<Duration>,

    failed_count: usize,

    passed_count: usize,

    report_name: Option<String>,

    workspace: Arc<RwLock<Workspace>>,
}

impl Runner {
    pub fn new(workspace: Workspace) -> Self {
        debug!(target: LOG_TARGET, "Creating action runner");

        Runner {
            bail: false,
            cached_count: 0,
            duration: None,
            failed_count: 0,
            passed_count: 0,
            report_name: None,
            workspace: Arc::new(RwLock::new(workspace)),
        }
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
        dep_graph: DepGraph,
        project_graph: ProjectGraph,
        context: Option<RunnerContext>,
    ) -> Result<ActionResults, RunnerError> {
        let start = Instant::now();
        let node_count = dep_graph.get_node_count();
        let batches = dep_graph.sort_batched_topological()?;
        let batches_count = batches.len();
        let dep_graph = Arc::new(RwLock::new(dep_graph));
        let project_graph = Arc::new(RwLock::new(project_graph));
        let context = Arc::new(RwLock::new(context.unwrap_or_default()));
        let emitter = Arc::new(RwLock::new(
            self.create_emitter(Arc::clone(&self.workspace)).await,
        ));
        let local_emitter = emitter.read().await;

        let mut results: ActionResults = vec![];

        let duration = start.elapsed();

        debug!(
            target: LOG_TARGET,
            "Finished running {} actions in {:?}", node_count, &duration
        );

        local_emitter
            .emit(Event::RunnerFinished {
                duration: &duration,
                cached_count: self.cached_count,
                failed_count: self.failed_count,
                passed_count: self.passed_count,
            })
            .await?;

        self.duration = Some(duration);

        Ok(results)
    }

    pub fn render_results(&self, results: &ActionResults) -> Result<(), MoonError> {
        let term = Term::buffered_stdout();
        term.write_line("")?;

        for result in results {
            let status = match result.status {
                ActionStatus::Passed
                | ActionStatus::Cached
                | ActionStatus::CachedFromRemote
                | ActionStatus::Skipped => color::success("pass"),
                ActionStatus::Failed | ActionStatus::FailedAndAbort => color::failure("fail"),
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

            // term.write_line(&format!(
            //     "{} {} {}",
            //     status,
            //     color::style(result.label.as_ref().unwrap()).bold(),
            //     color::muted(format!("({})", meta.join(", ")))
            // ))?;

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
            // if let Some(label) = &result.label {
            //     if compact && !label.contains("RunTarget") {
            //         continue;
            //     }
            // }

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
        context: Arc<RwLock<RunnerContext>>,
    ) -> Result<(), RunnerError> {
        if let Some(name) = &self.report_name {
            let workspace = self.workspace.read().await;
            let duration = self.duration.unwrap();
            let context = context.read().await;

            workspace
                .cache
                .create_json_report(name, RunReport::new(actions, &context, duration))?;
        }

        Ok(())
    }
}
