mod common;
pub mod endpoints;
pub mod graphql;

use crate::moonbase::common::*;
use crate::moonbase::endpoints::*;
use crate::moonbase_error::MoonbaseError;
use miette::IntoDiagnostic;
use moon_common::color;
use starbase_utils::fs;
use std::io;
use std::path::Path;
use std::path::PathBuf;
use tokio_util::codec::{BytesCodec, FramedRead};
use tracing::{debug, warn};

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
            "Unable to login to moonbase as no version control system was detected. We require VCS to infer the repository to sign in for!"
        );
    }

    pub async fn signin(secret_key: String, slug: String) -> Option<Moonbase> {
        debug!(
            "API keys detected, attempting to sign in to moonbase for repository {}",
            color::id(&slug),
        );

        let data: Result<Response<SigninResponse>, MoonbaseError> = post_request(
            "auth/repository/signin",
            SigninInput {
                organization_key: secret_key,
                repository: slug,
                repository_key: String::new(), // Remove from API
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
                    status,
                    "Failed to sign in to moonbase, please verify your API keys. Pipeline will still continue. Failure: {}", color::muted_light(message)
                );

                None
            }
            Err(error) => {
                warn!(
                    "Failed to sign in to moonbase, request has failed. Pipeline will still continue. Failure: {}", color::muted_light(error.to_string()),
                );

                None
            }
        }
    }

    pub async fn read_artifact(
        &self,
        hash: &str,
    ) -> miette::Result<Option<(Artifact, Option<String>)>> {
        let response = get_request(format!("artifacts/{hash}"), Some(&self.auth_token)).await?;

        match response {
            Response::Success(ArtifactResponse {
                artifact,
                presigned_url,
            }) => Ok(Some((artifact, presigned_url))),
            Response::Failure { message, status } => {
                if status == 404 {
                    Ok(None)
                } else {
                    Err(MoonbaseError::ArtifactCheckFailure {
                        hash: hash.to_owned(),
                        message,
                    }
                    .into())
                }
            }
        }
    }

    pub async fn write_artifact(
        &self,
        hash: &str,
        input: ArtifactWriteInput,
    ) -> miette::Result<(Artifact, Option<String>)> {
        let response =
            post_request(format!("artifacts/{hash}"), input, Some(&self.auth_token)).await?;

        match response {
            Response::Success(ArtifactResponse {
                artifact,
                presigned_url,
            }) => Ok((artifact, presigned_url)),
            Response::Failure { message, .. } => Err(MoonbaseError::ArtifactUploadFailure {
                hash: hash.to_owned(),
                message,
            }
            .into()),
        }
    }

    pub async fn download_artifact(
        &self,
        hash: &str,
        dest_path: &Path,
        download_url: &Option<String>,
    ) -> miette::Result<()> {
        let request = if let Some(url) = download_url {
            reqwest::Client::new().get(url)
        } else {
            reqwest::Client::new()
                .get(endpoint(format!("artifacts/{hash}/download")))
                .bearer_auth(&self.auth_token)
                .header("Accept", "application/json")
        };

        let response = request.send().await.into_diagnostic()?;
        let status = response.status();

        if status.is_success() {
            let mut contents = io::Cursor::new(response.bytes().await.into_diagnostic()?);
            let mut file = fs::create_file(dest_path)?;

            io::copy(&mut contents, &mut file).into_diagnostic()?;

            return Ok(());
        }

        Err(MoonbaseError::ArtifactDownloadFailure {
            hash: hash.to_string(),
            message: status
                .canonical_reason()
                .unwrap_or("Internal server error")
                .to_owned(),
        }
        .into())
    }

    pub async fn upload_artifact(
        &self,
        hash: String,
        path: PathBuf,
        upload_url: Option<String>,
        job_id: Option<i64>,
    ) -> miette::Result<()> {
        let file = tokio::fs::File::open(&path).await.into_diagnostic()?;
        let file_length = file
            .metadata()
            .await
            .map(|meta| meta.len())
            .unwrap_or_default();
        let file_stream = FramedRead::new(file, BytesCodec::new());

        let request = if let Some(url) = upload_url {
            reqwest::Client::new()
                .put(url)
                .header("Content-Length", file_length)
                .body(reqwest::Body::wrap_stream(file_stream))
        } else {
            reqwest::Client::new()
                .post(endpoint(format!("artifacts/{hash}/upload")))
                .body(reqwest::Body::wrap_stream(file_stream))
                .bearer_auth(&self.auth_token)
                .header("Accept", "application/json")
        };

        match request.send().await {
            Ok(response) => {
                let status = response.status();

                if status.is_success() {
                    self.mark_upload_complete(&hash, true, job_id).await?;

                    Ok(())
                } else {
                    self.mark_upload_complete(&hash, false, job_id).await?;

                    Err(MoonbaseError::ArtifactUploadFailure {
                        hash: hash.to_string(),
                        message: status
                            .canonical_reason()
                            .unwrap_or("Internal server error")
                            .to_owned(),
                    }
                    .into())
                }
            }
            Err(error) => {
                self.mark_upload_complete(&hash, false, job_id).await?;

                Err(MoonbaseError::ArtifactUploadFailure {
                    hash: hash.to_string(),
                    message: error.to_string(),
                }
                .into())
            }
        }
    }

    // Once the upload to cloud storage is complete, we need to mark the upload
    // as completed on our end, whether a success or failure!
    async fn mark_upload_complete(
        &self,
        hash: &str,
        success: bool,
        job_id: Option<i64>,
    ) -> Result<(), MoonbaseError> {
        let _: Response<EmptyData> = post_request(
            format!("artifacts/{hash}/complete"),
            ArtifactCompleteInput { job_id, success },
            Some(&self.auth_token),
        )
        .await?;

        Ok(())
    }
}
