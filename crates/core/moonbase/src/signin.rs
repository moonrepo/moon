use crate::common::{post_request, Response, LOG_TARGET};
use crate::errors::MoonbaseError;
use moon_logger::{color, warn};
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
) -> Result<Option<String>, MoonbaseError> {
    let data = post_request(
        "auth/repository/signin",
        SigninBody {
            organization_key: secret_key,
            repository: slug,
            repository_key: api_key,
        },
    )
    .await?;

    match data {
        Response::Failure { message, status } => {
            warn!(
                target: LOG_TARGET,
                "Failed to sign in to moonbase, please check your API keys. Process will still continue...\nFailure: {} ({})", color::muted_light(message), status
            );

            Ok(None)
        }
        Response::Success(SigninResponse { token }) => Ok(Some(token)),
    }
}
