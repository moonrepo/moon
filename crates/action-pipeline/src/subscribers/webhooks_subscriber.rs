use crate::event_emitter::{Event, Subscriber};
use async_trait::async_trait;
use moon_notifier::WebhooksNotifier;
use tracing::debug;

pub struct WebhooksSubscriber {
    notifier: WebhooksNotifier,
}

impl WebhooksSubscriber {
    pub fn new(url: &str, require_acknowledge: bool) -> Self {
        WebhooksSubscriber {
            notifier: WebhooksNotifier::new(url.to_owned(), require_acknowledge),
        }
    }
}

#[async_trait]
impl Subscriber for WebhooksSubscriber {
    async fn on_emit<'data>(&mut self, event: &Event<'data>) -> miette::Result<()> {
        self.notifier.notify(event.get_type(), event).await?;

        if matches!(event, Event::PipelineCompleted { .. }) {
            debug!("Waiting for webhook requests to finish");

            self.notifier.wait_for_requests().await;
        }

        Ok(())
    }
}
