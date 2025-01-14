use crate::event_emitter::{Event, Subscriber};
use async_trait::async_trait;
use moon_console::Console;
use moon_console::PipelineReportItem;
use std::sync::Arc;

pub struct ConsoleSubscriber {
    console: Arc<Console>,
    summarize: bool,
}

impl ConsoleSubscriber {
    pub fn new(console: Arc<Console>, summarize: bool) -> Self {
        Self { console, summarize }
    }
}

#[async_trait]
impl Subscriber for ConsoleSubscriber {
    async fn on_emit<'data>(&mut self, event: &Event<'data>) -> miette::Result<()> {
        match event {
            Event::PipelineStarted { action_nodes, .. } => {
                self.console.reporter.on_pipeline_started(action_nodes)?;
            }
            Event::PipelineCompleted {
                actions,
                duration,
                error_report,
                status,
                ..
            } => {
                let item = PipelineReportItem {
                    duration: *duration,
                    summarize: self.summarize,
                    status: **status,
                };

                self.console
                    .reporter
                    .on_pipeline_completed(actions, &item, *error_report)?;
            }
            Event::ActionStarted { action, .. } => {
                self.console.reporter.on_action_started(action)?;
            }
            Event::ActionCompleted {
                action,
                error_report,
                ..
            } => {
                self.console
                    .reporter
                    .on_action_completed(action, *error_report)?;
            }
            _ => {}
        };

        Ok(())
    }
}
