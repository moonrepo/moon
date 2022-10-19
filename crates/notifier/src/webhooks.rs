use moon_emitter::{Event, EventFlow, Subscriber};
use moon_error::MoonError;
use moon_logger::{color, error};
use moon_utils::time::chrono::prelude::*;
use moon_workspace::Workspace;
use serde::Serialize;
use tokio::task::JoinHandle;
use uuid::Uuid;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WebhookPayload<T: Serialize> {
    pub created_at: DateTime<Utc>,

    pub event: T,

    #[serde(rename = "type")]
    pub type_of: String,

    pub uuid: String,
}

pub async fn notify_webhook(
    url: String,
    body: String,
) -> Result<reqwest::Response, reqwest::Error> {
    reqwest::Client::new()
        .post(url)
        .body(body)
        .header("Accept", "application/json")
        .header("Content-Type", "application/json")
        .header("Connection", "keep-alive")
        .header("Keep-Alive", "timeout=30, max=120")
        .send()
        .await
}

pub struct WebhooksSubscriber {
    enabled: bool,
    requests: Vec<JoinHandle<()>>,
    url: String,
    uuid: String,
}

impl WebhooksSubscriber {
    pub fn new(url: String) -> Self {
        WebhooksSubscriber {
            enabled: true,
            requests: vec![],
            url,
            uuid: Uuid::new_v4().to_string(),
        }
    }
}

#[async_trait::async_trait]
impl Subscriber for WebhooksSubscriber {
    async fn on_emit<'a>(
        &mut self,
        event: &Event<'a>,
        _workspace: &Workspace,
    ) -> Result<EventFlow, MoonError> {
        if !self.enabled {
            return Ok(EventFlow::Continue);
        }

        let body = serde_json::to_string(&WebhookPayload {
            created_at: Utc::now(),
            event,
            type_of: event.get_type(),
            uuid: self.uuid.clone(),
        })
        .unwrap();

        // For the first event, we want to ensure that the webhook URL is valid
        // by sending the request and checking for a failure. If failed,
        // we will disable subsequent requests from being called.
        if matches!(event, Event::RunnerStarted { .. }) {
            let response = notify_webhook(self.url.to_owned(), body).await;

            if response.is_err() || !response.unwrap().status().is_success() {
                self.enabled = false;

                error!(
                    target: "moon:notifier:webhooks",
                    "Failed to send webhook event to {}. Subsequent webhook requests will be disabled.",
                    color::url(&self.url),
                );
            }

            // For every other event, we will make the request and ignore the result.
            // We will also avoid awaiting the request to not slow down the overall runner.
        } else {
            let url = self.url.to_owned();

            self.requests.push(tokio::spawn(async {
                let _ = notify_webhook(url, body).await;
            }));
        }

        // For the last event, we want to ensure that all webhook requests have
        // actually sent, otherwise, when the program exists all of these requests
        // will be dropped!
        if matches!(event, Event::RunnerAborted { .. })
            || matches!(event, Event::RunnerFinished { .. })
        {
            for future in self.requests.drain(0..) {
                let _ = future.await;
            }
        }

        Ok(EventFlow::Continue)
    }
}
