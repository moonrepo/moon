mod api;
mod common;
mod errors;

use std::path::Path;

use common::{get_request, post_request, Response};
use moon_logger::{color, warn};
use reqwest::multipart::{Form, Part};
use std::fs;

pub use api::*;
pub use errors::MoonbaseError;

use crate::common::{fetch, get_host};

const LOG_TARGET: &str = "moonbase";

#[derive(Debug)]
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

    pub async fn get_artifact(&self, hash: &str) -> Result<Option<Artifact>, MoonbaseError> {
        let response = get_request(format!("artifacts/{}", hash), Some(&self.auth_token)).await?;

        dbg!("get_artifact", &hash, &response);

        match response {
            Response::Success(ArtifactResponse { artifact }) => Ok(Some(artifact)),
            _ => Ok(None),
        }
    }

    pub async fn upload_artifact(
        &self,
        hash: &str,
        target: &str,
        path: &Path,
    ) -> Result<Option<Artifact>, MoonbaseError> {
        let form = Form::new()
            .text("repositoryId", self.repository_id.to_string())
            .text("target", target.to_owned())
            .part(
                "file",
                Part::bytes(fs::read(path).unwrap()).file_name(format!("{}.tar.gz", hash)),
            );

        let request = reqwest::Client::new()
            .post(format!("{}/artifacts/{}", get_host(), hash))
            .multipart(form);

        let response: Response<ArtifactResponse> = fetch(request, Some(&self.auth_token)).await?;

        dbg!("upload_artifact", &hash, &response);

        Ok(None)
    }
}
