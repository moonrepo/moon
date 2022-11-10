mod api;
mod common;
mod errors;

use common::{post_request, Response};
use moon_logger::{color, warn};

pub use api::*;
pub use errors::MoonbaseError;

const LOG_TARGET: &str = "moonbase";

pub struct Moonbase {
    auth_token: String,

    organization_id: i32,

    repository_id: i32,
}

impl Moonbase {
    pub fn new(auth_token: String, organization_id: i32, repository_id: i32) -> Self {
        Moonbase {
            auth_token,
            organization_id,
            repository_id,
        }
    }

    pub async fn signin(
        secret_key: String,
        api_key: String,
        slug: String,
    ) -> Result<Option<Moonbase>, MoonbaseError> {
        let data = post_request(
            "auth/repository/signin",
            SigninBody {
                organization_key: secret_key,
                repository: slug,
                repository_key: api_key,
            },
            None,
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
            Response::Success(SigninResponse {
                organization_id,
                repository_id,
                token,
            }) => Ok(Some(Moonbase::new(token, organization_id, repository_id))),
        }
    }
}
