use crate::event_emitter::{Event, Subscriber};
use async_trait::async_trait;
use moon_remote::RemoteService;
use std::sync::Arc;
use tracing::debug;

pub struct RemoteSubscriber {
    session: Arc<RemoteService>,
}

impl RemoteSubscriber {
    pub fn new(session: Arc<RemoteService>) -> Self {
        RemoteSubscriber { session }
    }
}

#[async_trait]
impl Subscriber for RemoteSubscriber {
    async fn on_emit<'data>(&mut self, event: &Event<'data>) -> miette::Result<()> {
        if matches!(event, Event::PipelineCompleted { .. }) {
            debug!("Waiting for in-flight remote service requests to finish");

            self.session.wait_for_requests().await;
        }

        Ok(())
    }
}
