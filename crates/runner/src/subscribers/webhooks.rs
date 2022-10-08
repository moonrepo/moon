use crate::emitter::Event;
use moon_contract::EventFlow;
use moon_error::MoonError;
use moon_logger::{color, error};
use moon_notifier::{notify_webhook, WebhookPayload};
use moon_workspace::Workspace;
use tokio::task::JoinHandle;

pub struct WebhooksSubscriber {
    enabled: bool,
    requests: Vec<JoinHandle<()>>,
    url: String,
}

impl WebhooksSubscriber {
    pub fn new(url: String) -> Self {
        WebhooksSubscriber {
            enabled: false,
            requests: vec![],
            url,
        }
    }

    pub async fn on_emit<'a>(
        &mut self,
        event: &Event<'a>,
        _workspace: &Workspace,
    ) -> Result<EventFlow, MoonError> {
        let url = self.url.to_owned();
        let body = serde_json::to_string(&WebhookPayload {
            type_of: event.get_type(),
            event,
        })
        .unwrap();

        // For the first event, we want to ensure that the webhook URL is valid
        // by sending the request and checking for a failure. If failed,
        // we will disable subsequent requests from being called.
        if matches!(event, Event::RunStarted { .. }) {
            if notify_webhook(url, body).await.is_err() {
                self.enabled = false;

                error!(
                    target: "moon:runner",
                    "Failed to send webhook event to {}. Subsequent webhook requests will be disabled.",
                    color::url(&self.url),
                );
            }

            // For every other event, we will make the request and ignore the result.
            // We will also avoid awaiting the request to not slow down the overall runner.
        } else if self.enabled {
            self.requests.push(tokio::spawn(async {
                let _ = notify_webhook(url, body).await;
            }));
        }

        // For the last event, we want to ensure that all webhook requests have
        // actually sent, otherwise, when the program exists all of these requests
        // will be dropped!
        if matches!(event, Event::RunAborted { .. }) || matches!(event, Event::RunFinished { .. }) {
            for future in self.requests.drain(0..) {
                let _ = future.await;
            }
        }

        Ok(EventFlow::Continue)
    }
}
