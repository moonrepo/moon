use ci_env::{CiEnvironment, get_environment};
use moon_common::color;
use moon_time::chrono::NaiveDateTime;
use moon_time::now_timestamp;
use serde::Serialize;
use starbase_utils::json;
use tokio::task::JoinHandle;
use tracing::{debug, trace, warn};
use uuid::Uuid;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WebhookPayload<'data, T: Serialize> {
    pub created_at: NaiveDateTime,

    pub environment: Option<&'data CiEnvironment>,

    pub event: T,

    #[serde(rename = "type")]
    pub type_of: &'data str,

    pub uuid: &'data str,
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

pub struct WebhooksNotifier {
    enabled: bool,
    environment: Option<CiEnvironment>,
    requests: Vec<JoinHandle<()>>,
    url: String,
    uuid: String,
    verified: bool,
}

impl WebhooksNotifier {
    pub fn new(url: String) -> Self {
        debug!("Creating webhooks notifier for {}", color::url(&url));

        WebhooksNotifier {
            enabled: true,
            environment: get_environment(),
            requests: vec![],
            uuid: if url.contains("127.0.0.1") {
                "XXXX-XXXX-XXXX-XXXX".into()
            } else {
                Uuid::new_v4().to_string()
            },
            url,
            verified: false,
        }
    }

    pub async fn notify<T: Serialize>(&mut self, name: &str, event: T) -> miette::Result<()> {
        if !self.enabled {
            return Ok(());
        }

        trace!("Posting webhook event {} to endpoint", color::id(name));

        let payload = WebhookPayload {
            created_at: now_timestamp(),
            environment: self.environment.as_ref(),
            event,
            type_of: name,
            uuid: &self.uuid,
        };
        let body = json::format(&payload, false)?;
        let url = self.url.to_owned();

        // For the first event, we want to ensure that the webhook URL is valid
        // by sending the request and checking for a failure. If failed,
        // we will disable subsequent requests from being called.
        if !self.verified {
            let response = notify_webhook(url, body).await;

            if response.is_err() || !response.unwrap().status().is_success() {
                self.enabled = false;

                warn!("Failed to send webhook event, subsequent webhook requests will be disabled");
            }

            self.verified = true;
        }
        // For every other event, we will make the request and ignore the result.
        // We will also avoid awaiting the request to not slow down the overall runner.
        else {
            self.requests.push(tokio::spawn(async {
                let _ = notify_webhook(url, body).await;
            }));
        }

        Ok(())
    }

    pub async fn wait_for_requests(&mut self) {
        for future in self.requests.drain(0..) {
            let _ = future.await;
        }
    }
}
