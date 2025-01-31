use crate::event_emitter::{Event, Subscriber};
use async_trait::async_trait;
use moon_action::ActionPipelineStatus;
use moon_cache::CacheEngine;
use std::sync::Arc;
use tracing::debug;

pub struct CleanupSubscriber {
    cache_engine: Arc<CacheEngine>,
    lifetime: String,
}

impl CleanupSubscriber {
    pub fn new(cache_engine: Arc<CacheEngine>, lifetime: &str) -> Self {
        CleanupSubscriber {
            cache_engine,
            lifetime: lifetime.to_owned(),
        }
    }
}

#[async_trait]
impl Subscriber for CleanupSubscriber {
    async fn on_emit<'data>(&mut self, event: &Event<'data>) -> miette::Result<()> {
        if matches!(
            event,
            Event::PipelineCompleted {
                status: ActionPipelineStatus::Completed,
                ..
            }
        ) {
            debug!("Cleaning stale cache");

            self.cache_engine.clean_stale_cache(&self.lifetime, false)?;
        }

        Ok(())
    }
}
