mod api;
mod common;
mod errors;
pub mod graphql;

use common::{endpoint, get_request, post_request, Response};
use moon_error::map_io_to_fs_error;
use moon_logger::{color, debug, warn};
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

    pub ci_insights_enabled: bool,

    #[allow(dead_code)]
    pub organization_id: i32,

    pub remote_caching_enabled: bool,

    pub repository_id: i32,
}

impl Moonbase {
    pub fn no_vcs_root() {
        warn!(
            target: LOG_TARGET,
            "Unable to login to moonbase as no version control system was detected. We require VCS to infer the repository to sign in for!"
        );
    }

    pub async fn signin(secret_key: String, access_key: String, slug: String) -> Option<Moonbase> {
        debug!(
            target: LOG_TARGET,
            "API keys detected, attempting to sign in to moonbase for repository {}",
            color::id(&slug),
        );

        let data = post_request(
            "auth/repository/signin",
            SigninInput {
                organization_key: secret_key,
                repository: slug,
                repository_key: access_key,
            },
            None,
        )
        .await;

        match data {
            Ok(Response::Success(SigninResponse {
                ci_insights,
                organization_id,
                remote_caching,
                repository_id,
                token,
            })) => Some(Moonbase {
                auth_token: token,
                ci_insights_enabled: ci_insights,
                organization_id,
                remote_caching_enabled: remote_caching,
                repository_id,
            }),
            Ok(Response::Failure { message, status }) => {
                warn!(
                    target: LOG_TARGET,
                    "Failed to sign in to moonbase, please verify your API keys. Pipeline will still continue. Failure: {} ({})", color::muted_light(message), status
                );

                None
            }
            Err(error) => {
                warn!(
                    target: LOG_TARGET,
                    "Failed to sign in to moonbase, request has failed. Pipeline will still continue. Failure: {} ", color::muted_light(error.to_string()),
                );

                None
            }
        }
    }

    pub async fn read_artifact(
        &self,
        hash: &str,
    ) -> Result<Option<(Artifact, Option<String>)>, MoonbaseError> {
        let response = get_request(format!("artifacts/{hash}"), Some(&self.auth_token)).await?;

        // dbg!("read_artifact", hash, &response);

        match response {
            Response::Success(ArtifactResponse {
                artifact,
                presigned_url,
            }) => Ok(Some((artifact, presigned_url))),
            Response::Failure { message, status } => {
                if status == 404 {
                    Ok(None)
                } else {
                    Err(MoonbaseError::ArtifactCheckFailure(
                        color::hash(hash),
                        message,
                    ))
                }
            }
        }
    }

    pub async fn write_artifact(
        &self,
        hash: &str,
        input: ArtifactWriteInput,
    ) -> Result<(Artifact, Option<String>), MoonbaseError> {
        // dbg!("write_artifact", hash, &input);

        let response =
            post_request(format!("artifacts/{hash}"), input, Some(&self.auth_token)).await?;

        // dbg!(&response);

        match response {
            Response::Success(ArtifactResponse {
                artifact,
                presigned_url,
            }) => Ok((artifact, presigned_url)),
            Response::Failure { message, .. } => Err(MoonbaseError::ArtifactUploadFailure(
                hash.to_string(),
                message,
            )),
        }
    }

    pub async fn download_artifact(
        &self,
        hash: &str,
        dest_path: &Path,
        download_url: &Option<String>,
    ) -> Result<(), MoonbaseError> {
        // dbg!("download_artifact", download_url);

        let request = if let Some(url) = download_url {
            reqwest::Client::new().get(url)
        } else {
            reqwest::Client::new()
                .get(endpoint(format!("artifacts/{hash}/download")))
                .bearer_auth(&self.auth_token)
                .header("Accept", "application/json")
        };

        let response = request.send().await?;
        let status = response.status();

        // dbg!(&status);

        if status.is_success() {
            let error_handler = |e: io::Error| map_io_to_fs_error(e, dest_path.to_path_buf());
            let mut contents = io::Cursor::new(response.bytes().await?);
            let mut file = std::fs::File::create(dest_path).map_err(error_handler)?;

            io::copy(&mut contents, &mut file).map_err(error_handler)?;

            return Ok(());
        }

        Err(MoonbaseError::ArtifactDownloadFailure(
            hash.to_string(),
            status
                .canonical_reason()
                .unwrap_or("Internal server error")
                .to_owned(),
        ))
    }
}

// This is a stand-alone function so that we may run it in a background Tokio thread,
// and not have to worry about lifetime and borrow issues.
pub async fn upload_artifact(
    auth_token: String,
    hash: String,
    path: PathBuf,
    upload_url: Option<String>,
    job_id: Option<i64>,
) -> Result<(), MoonbaseError> {
    let file = fs::File::open(&path)
        .await
        .map_err(|e| map_io_to_fs_error(e, path.to_path_buf()))?;
    let file_length = match file.metadata().await {
        Ok(meta) => meta.len(),
        Err(_) => 0,
    };
    let file_stream = FramedRead::new(file, BytesCodec::new());

    // dbg!("upload_artifact", &upload_url, &file_length);

    let request = if let Some(url) = upload_url {
        reqwest::Client::new()
            .put(url)
            .header("Content-Length", file_length)
            .body(Body::wrap_stream(file_stream))
    } else {
        reqwest::Client::new()
            .post(endpoint(format!("artifacts/{hash}/upload")))
            .body(Body::wrap_stream(file_stream))
            .bearer_auth(&auth_token)
            .header("Accept", "application/json")
    };

    match request.send().await {
        Ok(response) => {
            let status = response.status();

            if status.is_success() {
                mark_upload_complete(&auth_token, &hash, true, job_id).await?;

                Ok(())
            } else {
                mark_upload_complete(&auth_token, &hash, false, job_id).await?;

                Err(MoonbaseError::ArtifactUploadFailure(
                    hash.to_string(),
                    status
                        .canonical_reason()
                        .unwrap_or("Internal server error")
                        .to_owned(),
                ))
            }
        }
        Err(error) => {
            mark_upload_complete(&auth_token, &hash, false, job_id).await?;

            Err(MoonbaseError::ArtifactUploadFailure(
                hash.to_string(),
                error.to_string(),
            ))
        }
    }
}

// Once the upload to cloud storage is complete, we need to mark the upload
// as completed on our end, whether a success or failure!
async fn mark_upload_complete(
    auth_token: &str,
    hash: &str,
    success: bool,
    job_id: Option<i64>,
) -> Result<(), MoonbaseError> {
    let _: Response<EmptyData> = post_request(
        format!("artifacts/{hash}/complete"),
        ArtifactCompleteInput { job_id, success },
        Some(auth_token),
    )
    .await?;

    Ok(())
}
