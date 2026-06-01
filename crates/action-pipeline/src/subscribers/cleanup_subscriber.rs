use crate::event_emitter::{Event, Subscriber};
use async_trait::async_trait;
use moon_action::ActionPipelineStatus;
use moon_cache::CacheEngine;
use moon_daemon_client::DaemonClient;
use std::sync::Arc;
use tracing::debug;

pub struct CleanupSubscriber {
    cache_engine: Arc<CacheEngine>,
    daemon_client: Option<DaemonClient>,
    lifetime: String,
}

impl CleanupSubscriber {
    pub fn new(
        cache_engine: Arc<CacheEngine>,
        daemon_client: Option<DaemonClient>,
        lifetime: &str,
    ) -> Self {
        CleanupSubscriber {
            cache_engine,
            daemon_client,
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

            if let Some(daemon) = &mut self.daemon_client {
                daemon.clean_cache(&self.lifetime, false).await?;
            } else {
                self.cache_engine
                    .clean_stale_cache(&self.lifetime, false)
                    .await?;
            }
        }

        Ok(())
    }
}
