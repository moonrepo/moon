mod api;
mod common;
mod errors;

use common::{get_host, get_request, parse_response, post_request, Response};
use moon_error::map_io_to_fs_error;
use moon_logger::{color, warn};
use reqwest::multipart::{Form, Part};
use reqwest::Body;
use std::path::Path;
use tokio::fs;
use tokio_util::codec::{BytesCodec, FramedRead};

pub use api::*;
pub use errors::MoonbaseError;

const LOG_TARGET: &str = "moonbase";

#[derive(Clone, Debug)]
pub struct Moonbase {
    auth_token: String,

    organization_id: i32,

    repository_id: i32,
}

impl Moonbase {
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
                    "Failed to sign in to moonbase, please check your API keys. Pipeline will still continue... Failure: {} ({})", color::muted_light(message), status
                );

                Ok(None)
            }
            Response::Success(SigninResponse {
                organization_id,
                repository_id,
                token,
            }) => Ok(Some(Moonbase {
                auth_token: token,
                organization_id,
                repository_id,
            })),
        }
    }

    pub async fn get_artifact(&self, hash: &str) -> Result<Option<Artifact>, MoonbaseError> {
        let response = get_request(format!("artifacts/{}", hash), Some(&self.auth_token)).await?;

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
        let file = fs::File::open(path)
            .await
            .map_err(|e| map_io_to_fs_error(e, path.to_path_buf()))?;
        let file_name = match path.file_name() {
            Some(name) => name.to_string_lossy().to_string(),
            None => format!("{}.tar.gz", hash),
        };
        let file_stream = FramedRead::new(file, BytesCodec::new());

        let form = Form::new()
            .text("repository", self.repository_id.to_string())
            .text("target", target.to_owned())
            .part(
                "file",
                Part::stream(Body::wrap_stream(file_stream))
                    .file_name(file_name)
                    .mime_str("application/gzip")?,
            );

        let request = reqwest::Client::new()
            .post(format!("{}/artifacts/{}", get_host(), hash))
            .multipart(form)
            .bearer_auth(&self.auth_token)
            .header("Accept", "application/json");

        let response = request.send().await?;
        let data: Response<ArtifactResponse> = parse_response(response.text().await?)?;

        match data {
            Response::Success(ArtifactResponse { artifact }) => Ok(Some(artifact)),
            Response::Failure { message, status } => {
                warn!(
                    target: LOG_TARGET,
                    "Failed to upload artifact with hash {}. Failure: {} ({})",
                    color::symbol(hash),
                    color::muted_light(message),
                    status
                );

                Ok(None)
            }
        }
    }
}
