use crate::manifest::{Manifest, ManifestSource};
use crate::storage_backend::{BoxedStorageBackend, StorageBackend};
use miette::IntoDiagnostic;
use moon_blob::{BlobContent, BlobInput, Bytes};
use moon_common::Id;
use moon_config::{CacheConfig, RemoteConfig};
use moon_hash::Digest;
use rustc_hash::FxHashMap;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::SystemTime;
use tokio::task::{JoinHandle, JoinSet};
use tracing::{debug, trace, warn};

#[derive(Clone, Debug)]
pub struct CacheContext {
    pub cache_dir: PathBuf,
    pub cache_config: Arc<CacheConfig>,
    pub config_dir: PathBuf,
    pub remote_config: Arc<RemoteConfig>,
    pub remote_debug: bool,
    pub workspace_root: PathBuf,
}

#[derive(Debug)]
pub struct StorageOptions {
    pub only_backends: Vec<Id>,
    pub include_local: bool,
    pub include_remote: bool,
}

impl Default for StorageOptions {
    fn default() -> Self {
        Self {
            only_backends: vec![],
            include_local: true,
            include_remote: true,
        }
    }
}

#[derive(Debug, Default)]
pub struct Storage {
    background_tasks: Arc<Mutex<Vec<JoinHandle<miette::Result<()>>>>>,
    local_backends: Vec<BoxedStorageBackend>,
    remote_backends: Vec<BoxedStorageBackend>,
    options: StorageOptions,
}

impl Storage {
    pub fn with_options(&self, options: StorageOptions) -> Self {
        Self {
            background_tasks: Arc::clone(&self.background_tasks),
            local_backends: self.local_backends.clone(),
            remote_backends: self.remote_backends.clone(),
            options,
        }
    }

    pub fn add_local_backend(&mut self, backend: impl StorageBackend + 'static) {
        self.local_backends.push(Arc::new(backend));
    }

    pub fn add_remote_backend(&mut self, backend: impl StorageBackend + 'static) {
        self.remote_backends.push(Arc::new(backend));
    }

    pub async fn connect_backends(&self) -> miette::Result<()> {
        let mut set = JoinSet::new();

        for backend in self.get_backends() {
            let backend = Arc::clone(backend);

            set.spawn(async move {
                if let Err(error) = backend.connect().await {
                    warn!(
                        storage = backend.get_id().as_str(),
                        error = error.to_string(),
                        "Failed to connect to storage backend, disabling it"
                    );
                }
            });
        }

        while let Some(result) = set.join_next().await {
            if let Err(error) = result {
                warn!(
                    error = error.to_string(),
                    "Failed to connect storage backends"
                );
            }
        }

        Ok(())
    }

    pub fn get_backends(&self) -> Vec<&BoxedStorageBackend> {
        let mut backends = vec![];

        if self.options.only_backends.is_empty() {
            if self.options.include_local {
                backends.extend(self.local_backends.iter());
            }

            if self.options.include_remote {
                backends.extend(self.remote_backends.iter());
            }
        } else {
            backends.extend(self.local_backends.iter());
            backends.extend(self.remote_backends.iter());
            backends.retain(|backend| self.options.only_backends.contains(backend.get_id()));
        }

        backends
    }

    pub fn is_local_enabled(&self) -> bool {
        !self.local_backends.is_empty()
    }

    pub fn is_remote_enabled(&self) -> bool {
        !self.remote_backends.is_empty()
    }

    pub async fn load_manifest(&self, digest: &Digest) -> miette::Result<Option<ManifestSource>> {
        trace!(hash = digest.hash.as_str(), "Checking for a cache manifest");

        for backend in self.get_backends() {
            if !backend.is_enabled() {
                continue;
            }

            if let Some(manifest) = backend.retrieve_manifest(digest.to_owned()).await? {
                trace!(
                    storage = backend.get_id().as_str(),
                    hash = digest.hash.as_str(),
                    files = manifest.files.len(),
                    symlinks = manifest.symlinks.len(),
                    exit_code = manifest.exit_code,
                    "Cache hit on manifest"
                );

                return Ok(Some(ManifestSource {
                    backend: Arc::clone(backend),
                    manifest,
                    remote: self
                        .remote_backends
                        .iter()
                        .any(|remote| remote.get_id() == backend.get_id()),
                }));
            }
        }

        trace!(hash = digest.hash.as_str(), "Cache miss on manifest");

        Ok(None)
    }

    pub async fn archive_manifest(
        &self,
        digest: &Digest,
        manifest: Manifest,
    ) -> miette::Result<()> {
        let mut background_tasks = self.background_tasks.lock().unwrap();

        trace!(
            hash = digest.hash.as_str(),
            files = manifest.files.len(),
            symlinks = manifest.symlinks.len(),
            exit_code = manifest.exit_code,
            "Archiving cache manifest"
        );

        // Store the manifest in all backends in parallel, but if any fail,
        // continue storing the rest for failover/redundancy in the future
        for backend in self.get_backends() {
            if !backend.is_enabled() {
                continue;
            }

            background_tasks.push(tokio::spawn(Box::pin(archive_manifest_in_backend(
                Arc::clone(backend),
                digest.to_owned(),
                manifest.clone(),
            ))));
        }

        trace!(
            hash = digest.hash.as_str(),
            files = manifest.files.len(),
            symlinks = manifest.symlinks.len(),
            exit_code = manifest.exit_code,
            "Archived cache manifest (in background queue)"
        );

        Ok(())
    }

    pub async fn hydrate_manifest(
        &self,
        digest: &Digest,
        manifest_source: ManifestSource,
    ) -> miette::Result<Option<Manifest>> {
        let ManifestSource {
            mut manifest,
            backend: original_backend,
            ..
        } = manifest_source;
        let mut backends = VecDeque::from_iter(self.get_backends());
        let mut count = 1;

        trace!(hash = digest.hash.as_str(), "Hydrating cache manifest");

        // Hydrate the manifest from the backend it was originally loaded from,
        // as that's the most likely to have all the blobs available
        hydrate_manifest_from_backend(&original_backend, digest, &mut manifest).await?;

        // If the original backend doesn't have all the blobs available,
        // we should attempt to hydrate from the other backends,
        // and also copy the missing blobs to the original backend
        while !manifest.is_hydrated()
            && let Some(backend) = backends.pop_front()
            && backend.is_enabled()
        {
            if backend.get_id() == original_backend.get_id() {
                continue;
            }

            count += 1;

            hydrate_manifest_from_backend_and_copy_to_original(
                &original_backend,
                backend,
                digest,
                &mut manifest,
            )
            .await?;
        }

        // If the manifest is fully hydrated, return it, otherwise return None to
        // indicate it couldn't be fully hydrated, and should re-run
        if manifest.is_hydrated() {
            trace!(
                hash = digest.hash.as_str(),
                "Hydrating cache manifest from {count} storage backends"
            );

            return Ok(Some(manifest));
        }

        trace!(
            hash = digest.hash.as_str(),
            "Failed to hydrate cache manifest as some blobs were missing"
        );

        Ok(None)
    }

    pub async fn wait_for_background_tasks(&self) -> miette::Result<()> {
        let background_tasks = {
            self.background_tasks
                .lock()
                .unwrap()
                .drain(0..)
                .collect::<Vec<_>>()
        };

        for handle in background_tasks {
            let _ = handle.await.into_diagnostic()?;
        }

        Ok(())
    }
}

async fn archive_manifest_in_backend(
    backend: BoxedStorageBackend,
    digest: Digest,
    mut manifest: Manifest,
) -> miette::Result<()> {
    let blob_sources = manifest.collect_blob_inputs();
    let initial_count = blob_sources.len();

    if !blob_sources.is_empty() {
        trace!(
            storage = backend.get_id().as_str(),
            hash = digest.hash.as_str(),
            "Storing {initial_count} blobs"
        );

        manifest.upload_started_at = Some(SystemTime::now());

        // Before we store the manifest, we should ensure all associated blobs are stored.
        // This ensures we don't end up with dangling manifests that reference missing blobs.
        let uploaded = Arc::clone(&backend)
            .store_blobs_batched(digest.clone(), blob_sources)
            .await?;

        manifest.upload_completed_at = Some(SystemTime::now());

        trace!(
            storage = backend.get_id().as_str(),
            hash = digest.hash.as_str(),
            "Stored {} of {initial_count} blobs",
            uploaded.len()
        );

        if uploaded.is_empty() {
            return Ok(());
        }
    }

    if backend.get_capabilities().store_manifests {
        if let Err(error) = backend.store_manifest(digest.clone(), manifest).await {
            warn!(
                storage = backend.get_id().as_str(),
                hash = digest.hash.as_str(),
                error = error.to_string(),
                "Failed to store cache manifest",
            );
        }
    } else {
        debug!(
            storage = backend.get_id().as_str(),
            hash = digest.hash.as_str(),
            "Successfully stored blobs but skipping cache manifest, as storage backend capabilities have it explicitly disabled",
        );
    }

    Ok(())
}

async fn hydrate_manifest_from_backend(
    backend: &BoxedStorageBackend,
    digest: &Digest,
    manifest: &mut Manifest,
) -> miette::Result<FxHashMap<Digest, Bytes>> {
    let blob_digests = manifest.collect_unhydrated_blob_digests();
    let initial_count = blob_digests.len();

    trace!(
        storage = backend.get_id().as_str(),
        hash = digest.hash.as_str(),
        "Retrieving {initial_count} blobs"
    );

    // Retrieve all blobs for digests that have yet to be hydrated
    let blobs_map = Arc::clone(backend)
        .retrieve_blobs_batched(digest.clone(), blob_digests)
        .await?
        .into_iter()
        .map(|blob| (blob.digest, blob.bytes))
        .collect::<FxHashMap<_, _>>();

    trace!(
        storage = backend.get_id().as_str(),
        hash = digest.hash.as_str(),
        "Retrieved {} of {initial_count} blobs",
        blobs_map.len()
    );

    // And then copy their data into the manifest
    manifest.hydrate(&blobs_map);

    Ok(blobs_map)
}

async fn hydrate_manifest_from_backend_and_copy_to_original(
    original_backend: &BoxedStorageBackend,
    backend: &BoxedStorageBackend,
    digest: &Digest,
    manifest: &mut Manifest,
) -> miette::Result<()> {
    // Collect the unhydrated blob digests from the manifest before hydrating,
    // so we can compare which are missing and attempt to copy them
    let unhydrated_digests = manifest.collect_unhydrated_blob_digests();
    let blobs_map = hydrate_manifest_from_backend(backend, digest, manifest).await?;

    // Loop through and create the blob inputs for the missing blobs
    let mut blob_inputs = vec![];

    for digest in unhydrated_digests {
        if let Some(bytes) = blobs_map.get(&digest) {
            blob_inputs.push(BlobInput {
                content: BlobContent::Inline(bytes.to_owned()),
                digest,
            });
        }
    }

    // Then store them in the original backend in which they were missing
    if !blob_inputs.is_empty() {
        trace!(
            to_storage = original_backend.get_id().as_str(),
            from_storage = backend.get_id().as_str(),
            hash = digest.hash.as_str(),
            "Copying {} missing blobs to original storage backend",
            blob_inputs.len()
        );

        Arc::clone(original_backend)
            .store_blobs_batched(digest.to_owned(), blob_inputs)
            .await?;
    }

    Ok(())
}
