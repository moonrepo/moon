use crate::event_emitter::{Event, Subscriber};
use crate::reports::estimate::Estimate;
use async_trait::async_trait;
use moon_action::Action;
use moon_action_context::ActionContext;
use moon_cache::CacheEngine;
use serde::Serialize;
use std::sync::Arc;
use std::time::Duration;
use tracing::debug;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunReport<'data> {
    pub actions: &'data [Action],

    pub context: &'data ActionContext,

    /// How long the pipeline took to execute all actions.
    pub duration: &'data Duration,

    /// Estimates around how much time was saved using moon,
    /// compared to another product or baseline.
    pub comparison_estimate: Estimate,
}

pub struct ReportsSubscriber {
    cache_engine: Arc<CacheEngine>,
    action_context: Arc<ActionContext>,
}

impl ReportsSubscriber {
    pub fn new(cache_engine: Arc<CacheEngine>, action_context: Arc<ActionContext>) -> Self {
        ReportsSubscriber {
            cache_engine,
            action_context,
        }
    }
}

#[async_trait]
impl Subscriber for ReportsSubscriber {
    async fn on_emit<'data>(&mut self, event: &Event<'data>) -> miette::Result<()> {
        if let Event::PipelineCompleted {
            actions, duration, ..
        } = event
        {
            debug!("Creating run report");

            let estimate = Estimate::calculate(actions, duration);

            let report = RunReport {
                actions,
                context: &self.action_context,
                duration,
                comparison_estimate: estimate,
            };

            self.cache_engine.write("runReport.json", &report)?;
        }

        Ok(())
    }
}
