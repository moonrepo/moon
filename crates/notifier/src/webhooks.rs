use ci_env::{CiEnvironment, get_environment};
use moon_common::color;
use moon_time::chrono::NaiveDateTime;
use moon_time::now_timestamp;
use serde::Serialize;
use starbase_utils::json;
use std::env;
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

    pub trace: &'data str,
}

pub async fn notify_webhook(
    url: String,
    body: String,
    require_acknowledge: bool,
) -> Result<reqwest::Response, reqwest::Error> {
    let response = reqwest::Client::new()
        .post(url)
        .body(body)
        .header("Accept", "application/json")
        .header("Content-Type", "application/json")
        .header("Connection", "keep-alive")
        .header("Keep-Alive", "timeout=30, max=120")
        .send()
        .await?;

    if require_acknowledge && !response.status().is_success() {
        return Err(response.error_for_status().unwrap_err());
    }

    Ok(response)
}

pub struct WebhooksNotifier {
    enabled: bool,
    environment: Option<CiEnvironment>,
    requests: Vec<JoinHandle<()>>,
    url: String,
    uuid: String,
    trace: String,
    verified: bool,
    require_acknowledge: bool,
}

impl WebhooksNotifier {
    pub fn new(url: String, require_acknowledge: bool) -> Self {
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
            trace: env::var("MOON_TRACE_ID").unwrap_or_default(),
            url,
            verified: false,
            require_acknowledge,
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
            trace: &self.trace,
        };
        let body = json::format(&payload, false)?;
        let url = self.url.to_owned();
        let require_acknowledge = self.require_acknowledge.to_owned();

        // For the first event, we want to ensure that the webhook URL is valid
        // by sending the request and checking for a failure. If failed,
        // we will disable subsequent requests from being called.
        if !self.verified {
            let response = notify_webhook(url, body, require_acknowledge).await;

            if response.is_err() || !response.unwrap().status().is_success() {
                self.enabled = false;

                warn!("Failed to send webhook event, subsequent webhook requests will be disabled");
            }

            self.verified = true;
        }
        // For every other event, we will make the request and ignore the result.
        // We will also avoid awaiting the request to not slow down the overall runner.
        else if require_acknowledge {
            let _ = notify_webhook(url.clone(), body, true).await;
        } else {
            self.requests.push(tokio::spawn(async move {
                let _ = notify_webhook(url, body, false).await;
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
