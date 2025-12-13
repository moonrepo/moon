use crate::event_emitter::{Event, Subscriber};
use async_trait::async_trait;
use moon_console::{Console, Level, PipelineReportItem};
use std::sync::Arc;

pub struct ConsoleSubscriber {
    console: Arc<Console>,
    summary: Option<Level>,
}

impl ConsoleSubscriber {
    pub fn new(console: Arc<Console>, summary: Option<Level>) -> Self {
        Self { console, summary }
    }
}

#[async_trait]
impl Subscriber for ConsoleSubscriber {
    async fn on_emit<'data>(&mut self, event: &Event<'data>) -> miette::Result<()> {
        match event {
            Event::PipelineStarted { action_nodes, .. } => {
                self.console.on_pipeline_started(action_nodes)?;
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
                    summary: self.summary.clone(),
                    status: **status,
                };

                self.console
                    .on_pipeline_completed(actions, &item, *error_report)?;
            }
            Event::ActionStarted { action, .. } => {
                self.console.on_action_started(action)?;
            }
            Event::ActionCompleted {
                action,
                error_report,
                ..
            } => {
                self.console.on_action_completed(action, *error_report)?;
            }
            _ => {}
        };

        Ok(())
    }
}
