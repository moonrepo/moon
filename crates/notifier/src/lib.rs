pub mod subscribers;

use moon_utils::time::chrono::prelude::*;
use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WebhookPayload<T: Serialize> {
    pub created_at: DateTime<Utc>,

    pub event: T,

    #[serde(rename = "type")]
    pub type_of: String,
}

pub async fn notify_webhook(
    url: String,
    body: String,
) -> Result<reqwest::Response, reqwest::Error> {
    reqwest::Client::new().post(url).body(body).send().await
}
