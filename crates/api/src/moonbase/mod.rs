mod common;
pub mod endpoints;

use crate::moonbase::common::*;
use crate::moonbase::endpoints::*;
use crate::moonbase_error::MoonbaseError;
use miette::IntoDiagnostic;
use moon_common::color;
use rustc_hash::FxHashMap;
use starbase_utils::fs;
use std::io;
use std::path::Path;
use std::sync::{Arc, OnceLock};
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tokio_util::codec::{BytesCodec, FramedRead};
use tracing::instrument;
use tracing::{debug, info, warn};

static INSTANCE: OnceLock<Arc<Moonbase>> = OnceLock::new();

#[derive(Clone, Debug)]
pub struct Moonbase {
    pub auth_token: String,

    #[allow(dead_code)]
    pub organization_id: i32,

    pub remote_caching_enabled: bool,

    pub repository_id: i32,

    download_urls: Arc<RwLock<FxHashMap<String, Option<String>>>>,

    upload_requests: Arc<RwLock<Vec<JoinHandle<()>>>>,
}

impl Moonbase {
    pub fn no_vcs_root() {
        warn!(
            "Unable to login to moonbase as no version control system was detected. We require VCS to infer the repository to sign in for!"
        );
    }

    pub fn session() -> Option<Arc<Moonbase>> {
        INSTANCE.get().cloned()
    }

    #[instrument(skip_all)]
    pub async fn signin(secret_key: String, slug: String) -> Option<Arc<Moonbase>> {
        if let Some(instance) = Self::session() {
            return Some(instance);
        }

        info!(
            "API keys detected, attempting to sign in to moonbase for repository {}",
            color::id(&slug),
        );

        let data: Result<Response<SigninResponse>, MoonbaseError> = post_request(
            "auth/repository/signin",
            SigninInput {
                organization_key: secret_key,
                repository: slug,
            },
            None,
        )
        .await;

        match data {
            Ok(Response::Success(SigninResponse {
                organization_id,
                remote_caching,
                repository_id,
                token,
                ..
            })) => {
                debug!("Sign in successful!");

                let instance = Arc::new(Moonbase {
                    auth_token: token,
                    organization_id,
                    remote_caching_enabled: remote_caching,
                    repository_id,
                    download_urls: Arc::new(RwLock::new(FxHashMap::default())),
                    upload_requests: Arc::new(RwLock::new(vec![])),
                });

                let _ = INSTANCE.set(Arc::clone(&instance));

                Some(instance)
            }
            Ok(Response::Failure { message, status }) => {
                warn!(
                    status,
                    "Failed to sign in to moonbase, please verify your API keys. Pipeline will still continue",
                );
                warn!("Failure: {}", color::muted_light(message));

                None
            }
            Err(error) => {
                warn!(
                    "Failed to sign in to moonbase, request has failed. Pipeline will still continue",
                );
                warn!("Failure: {}", color::muted_light(error.to_string()));

                None
            }
        }
    }

    #[instrument(skip(self))]
    pub async fn read_artifact(
        &self,
        hash: &str,
    ) -> miette::Result<Option<(Artifact, Option<String>)>> {
        let response = get_request(format!("artifacts/{hash}"), Some(&self.auth_token)).await?;

        match response {
            Response::Success(ArtifactResponse {
                artifact,
                presigned_url,
            }) => {
                self.download_urls
                    .write()
                    .await
                    .insert(artifact.hash.clone(), presigned_url.to_owned());

                Ok(Some((artifact, presigned_url)))
            }
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

    #[instrument(skip(self, input))]
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

    #[instrument(skip(self))]
    pub async fn download_artifact_from_remote_storage(
        &self,
        hash: &str,
        dest_path: &Path,
    ) -> miette::Result<()> {
        if !self.remote_caching_enabled {
            return Ok(());
        }

        if let Some(download_url) = self.download_urls.read().await.get(hash) {
            debug!(
                hash,
                archive_file = ?dest_path,
                "Downloading archive (artifact) from remote storage",
            );

            if let Err(error) = self.download_artifact(hash, dest_path, download_url).await {
                warn!(
                    hash,
                    archive_file = ?dest_path,
                    "Failed to download archive from remote storage: {}",
                    color::muted_light(error.to_string()),
                );
            }
        }

        Ok(())
    }

    #[instrument(skip(self))]
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

    #[instrument(skip(self))]
    pub async fn upload_artifact_to_remote_storage(
        &self,
        hash: &str,
        src_path: &Path,
        target_id: &str,
    ) -> miette::Result<()> {
        if !self.remote_caching_enabled {
            return Ok(());
        }

        let size = match fs::metadata(src_path) {
            Ok(meta) => meta.len(),
            Err(_) => 0,
        };

        debug!(
            hash,
            archive_file = ?src_path,
            size,
            "Uploading archive (artifact) to remote storage",
        );

        // Create the database record then upload to cloud storage
        let Ok((_, presigned_url)) = self
            .write_artifact(
                hash,
                ArtifactWriteInput {
                    target: target_id.to_owned(),
                    size: size as usize,
                },
            )
            .await
        else {
            return Ok(());
        };

        // Run this in the background so we don't slow down the pipeline
        // while waiting for very large archives to upload
        let moonbase = self.clone();
        let hash = hash.to_owned();
        let src_path = src_path.to_owned();

        self.upload_requests
            .write()
            .await
            .push(tokio::spawn(async move {
                if let Err(error) = moonbase
                    .upload_artifact(&hash, &src_path, presigned_url)
                    .await
                {
                    warn!(
                        hash,
                        archive_file = ?src_path,
                        "Failed to upload archive to remote storage: {}",
                        color::muted_light(error.to_string()),
                    );
                }
            }));

        Ok(())
    }

    #[instrument(skip(self))]
    pub async fn upload_artifact(
        &self,
        hash: &str,
        src_path: &Path,
        upload_url: Option<String>,
    ) -> miette::Result<()> {
        let file = tokio::fs::File::open(src_path).await.into_diagnostic()?;
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
                    self.mark_upload_complete(hash, true).await?;

                    Ok(())
                } else {
                    self.mark_upload_complete(hash, false).await?;

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
                self.mark_upload_complete(hash, false).await?;

                Err(MoonbaseError::ArtifactUploadFailure {
                    hash: hash.to_string(),
                    message: error.to_string(),
                }
                .into())
            }
        }
    }

    pub async fn wait_for_requests(&self) {
        let mut requests = self.upload_requests.write().await;

        for future in requests.drain(0..) {
            // We can ignore the errors because we handle them in
            // the tasks above by logging to the console
            let _ = future.await;
        }
    }

    // Once the upload to cloud storage is complete, we need to mark the upload
    // as completed on our end, whether a success or failure!
    async fn mark_upload_complete(&self, hash: &str, success: bool) -> Result<(), MoonbaseError> {
        let _: Response<EmptyData> = post_request(
            format!("artifacts/{hash}/complete"),
            ArtifactCompleteInput {
                job_id: None,
                success,
            },
            Some(&self.auth_token),
        )
        .await?;

        Ok(())
    }
}
