use crate::common::{Response, LOG_TARGET};
use moon_logger::warn;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct SigninBody {
    organization_key: String,
    repository: String,
    repository_key: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SigninResponse {
    token: String,
}

pub async fn signin(
    secret_key: String,
    api_key: String,
    slug: String,
) -> Result<Option<String>, reqwest::Error> {
    let body = serde_json::to_string(&SigninBody {
        organization_key: secret_key,
        repository: slug,
        repository_key: api_key,
    })
    .unwrap();

    let response = reqwest::Client::new()
        .post("http://127.0.0.1:8000/auth/repository/signin")
        .body(body)
        .header("Accept", "application/json")
        .header("Content-Type", "application/json")
        .header("Connection", "keep-alive")
        .header("Keep-Alive", "timeout=30, max=120")
        .send()
        .await?;
    let text = response.text().await?;
    let data: Response<SigninResponse> = serde_json::from_str(&text).unwrap();

    match data {
        Response::Failure { message, .. } => {
            warn!(
                target: LOG_TARGET,
                "Failed to sign in to moonbase, please check your API keys. Failure: {}", message
            );

            Ok(None)
        }
        Response::Success(SigninResponse { token }) => Ok(Some(token)),
    }
}
