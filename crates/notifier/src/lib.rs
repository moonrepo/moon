use reqwest;
use serde::Serialize;

#[derive(Serialize)]
pub struct WebhookPayload<T: Serialize> {
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
