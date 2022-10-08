use crate::{emitter::Event, RunnerError};
use moon_contract::EventFlow;
use moon_error::MoonError;
use moon_logger::error;
use moon_workspace::Workspace;
use reqwest;
use serde::Serialize;

#[derive(Serialize)]
struct WebhookPayload<T: Serialize> {
    event: T,
    #[serde(rename = "type")]
    type_of: String,
}

pub struct WebhooksSubscriber {
    client: reqwest::Client,
    enabled: bool,
    url: String,
}

impl WebhooksSubscriber {
    pub fn new(url: String) -> Self {
        WebhooksSubscriber {
            client: reqwest::Client::new(),
            enabled: false,
            url,
        }
    }

    pub async fn on_emit<'a>(
        &mut self,
        event: &Event<'a>,
        _workspace: &Workspace,
    ) -> Result<EventFlow, MoonError> {
        let body = serde_json::to_string(&WebhookPayload {
            type_of: event.get_type(),
            event,
        })
        .unwrap();

        // For the first event, we want to ensure that the webhook URL is valid
        // by making the request and checking for a failure. If failed,
        // we will disable subsequent requests from being called.
        if matches!(event, Event::RunStarted { .. }) {
            if let Err(_) = self.send(&body).await {
                self.enabled = false;

                error!(
                    target: "moon:runner",
                    "Failed to send webhook event to <url>{}</url>. Subsequent webhook requests will be disabled.",
                    self.url,
                );
            }

            // For every other event, we will make the request and ignore the result.
            // We will also avoid awaiting the request to not slow down the overall runner.
        } else {
            let _ = tokio::spawn(async {
                self.send(&body);
            });
        }

        Ok(EventFlow::Continue)
    }

    async fn send<'a>(&self, body: &str) -> Result<(), RunnerError> {
        if self.enabled {
            self.client
                .post(&self.url)
                .body(body)
                .send()
                .await
                .map_err(|e| MoonError::Generic(e.to_string()))?;
        }

        Ok(())
    }
}
