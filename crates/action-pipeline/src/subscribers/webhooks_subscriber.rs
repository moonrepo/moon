use std::sync::Arc;

use crate::event_emitter::{Event, Subscriber};
use async_trait::async_trait;
use moon_notifier::WebhooksNotifier;
use moon_process::ProcessRegistry;
use tracing::{debug, error};

pub struct WebhooksSubscriber {
    notifier: WebhooksNotifier,
    acknowledge: bool,
    receiver: Arc<ProcessRegistry>,
}

impl WebhooksSubscriber {
    pub fn new(url: &str, acknowledge: &bool, receiver: Arc<ProcessRegistry>) -> Self {
        WebhooksSubscriber {
            notifier: WebhooksNotifier::new(url.to_owned()),
            acknowledge: acknowledge.to_owned(),
            receiver,
        }
    }
}

#[async_trait]
impl Subscriber for WebhooksSubscriber {
    async fn on_emit<'data>(&mut self, event: &Event<'data>) -> miette::Result<()> {
        if self.acknowledge {
            let _ = self.notifier.wait_for_requests().await;
            if !self.notifier.verified {
                self.receiver.terminate_running();
                error!("Webhook notifier was not successful, abort pipeline!");
            }
        } else {
            self.notifier.notify(event.get_type(), event).await?;

            if matches!(event, Event::PipelineCompleted { .. }) {
                debug!("Waiting for webhook requests to finish");

                self.notifier.wait_for_requests().await;
            }
        }

        Ok(())
    }
}
