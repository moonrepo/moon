mod api;
mod common;
mod errors;

use common::{get_host, get_request, parse_response, post_request, Response};
use moon_error::map_io_to_fs_error;
use moon_logger::{color, debug, trace, warn};
use reqwest::multipart::{Form, Part};
use reqwest::Body;
use std::io;
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio_util::codec::{BytesCodec, FramedRead};

pub use api::*;
pub use errors::MoonbaseError;

const LOG_TARGET: &str = "moonbase";

#[derive(Clone, Debug)]
pub struct Moonbase {
    pub auth_token: String,

    #[allow(dead_code)]
    pub organization_id: i32,

    #[allow(dead_code)]
    pub repository_id: i32,
}

impl Moonbase {
    pub async fn signin(secret_key: String, api_key: String, slug: String) -> Option<Moonbase> {
        debug!(
            target: LOG_TARGET,
            "API keys detected, attempting to sign in to moonbase for repository {}",
            color::id(&slug),
        );

        let data = post_request(
            "auth/repository/signin",
            SigninBody {
                organization_key: secret_key,
                repository: slug,
                repository_key: api_key,
            },
            None,
        )
        .await;

        match data {
            Ok(Response::Success(SigninResponse {
                organization_id,
                repository_id,
                token,
            })) => Some(Moonbase {
                auth_token: token,
                organization_id,
                repository_id,
            }),
            Ok(Response::Failure { message, status }) => {
                warn!(
                    target: LOG_TARGET,
                    "Failed to sign in to moonbase, please verify your API keys. Pipeline will still continue... Failure: {} ({})", color::muted_light(message), status
                );

                None
            }
            Err(error) => {
                warn!(
                    target: LOG_TARGET,
                    "Failed to sign in to moonbase, request has failed. Pipeline will still continue... Failure: {} ", color::muted_light(error.to_string()),
                );

                None
            }
        }
    }

    pub async fn get_artifact(&self, hash: &str) -> Result<Option<Artifact>, MoonbaseError> {
        let response = get_request(format!("artifacts/{}", hash), Some(&self.auth_token)).await?;

        match response {
            Response::Success(ArtifactResponse { artifact }) => Ok(Some(artifact)),
            Response::Failure { message, status } => {
                if status == 404 {
                    Ok(None)
                } else {
                    Err(MoonbaseError::ArtifactCheckFailure(
                        hash.to_string(),
                        message,
                    ))
                }
            }
        }
    }

    pub async fn download_artifact(
        &self,
        hash: &str,
        dest_path: &Path,
    ) -> Result<(), MoonbaseError> {
        let response = reqwest::Client::new()
            .get(format!("{}/artifacts/{}/download", get_host(), hash))
            .bearer_auth(&self.auth_token)
            .header("Accept", "application/json")
            .send()
            .await?;
        let status = response.status();

        if status.is_success() {
            let error_handler = |e: io::Error| map_io_to_fs_error(e, dest_path.to_path_buf());
            let mut contents = io::Cursor::new(response.bytes().await?);
            let mut file = std::fs::File::create(dest_path).map_err(error_handler)?;

            io::copy(&mut contents, &mut file).map_err(error_handler)?;

            return Ok(());
        }

        let data: Response<ArtifactResponse> = parse_response(response.text().await?)?;
        let error_message = match data {
            Response::Failure { message, .. } => message,
            _ => "Unknown failure!".into(),
        };

        Err(MoonbaseError::ArtifactDownloadFailure(
            hash.to_string(),
            error_message,
        ))
    }
}

// This is a stand-alone function so that we may run it in the background in a tokio thread,
// and not have to worry about lifetime and borrow issues.
pub async fn upload_artifact(
    auth_token: String,
    repository_id: i32,
    hash: String,
    target: String,
    path: PathBuf,
) -> Result<Artifact, MoonbaseError> {
    let file = fs::File::open(&path)
        .await
        .map_err(|e| map_io_to_fs_error(e, path.to_path_buf()))?;
    let file_name = match path.file_name() {
        Some(name) => name.to_string_lossy().to_string(),
        None => format!("{}.tar.gz", hash),
    };
    let file_size = match file.metadata().await {
        Ok(meta) => meta.len(),
        Err(_) => 0,
    };
    let file_stream = FramedRead::new(file, BytesCodec::new());

    let form = Form::new()
        .text("repository", repository_id.to_string())
        .text("target", target.to_owned())
        .part(
            "file",
            Part::stream(Body::wrap_stream(file_stream))
                .file_name(file_name.clone())
                .mime_str("application/gzip")?,
        );

    let request = reqwest::Client::new()
        .post(format!("{}/artifacts/{}", get_host(), hash))
        .multipart(form)
        .bearer_auth(auth_token)
        .header("Accept", "application/json");

    trace!(
        target: LOG_TARGET,
        "Uploading artifact {} ({} bytes) to remote cache",
        color::file(&file_name),
        if file_size == 0 {
            "unknown".to_owned()
        } else {
            file_size.to_string()
        }
    );

    let response = request.send().await?;
    let data: Response<ArtifactResponse> = parse_response(response.text().await?)?;

    match data {
        Response::Success(ArtifactResponse { artifact }) => Ok(artifact),
        Response::Failure { message, .. } => Err(MoonbaseError::ArtifactUploadFailure(
            hash.to_string(),
            message,
        )),
    }
}
