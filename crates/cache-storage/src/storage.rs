use crate::manifest::{Manifest, ManifestSource};
use crate::storage_backend::{BoxedStorageBackend, StorageBackend};
use moon_blob::{BlobCleanStats, BlobContent, BlobInput, BlobOutput};
use moon_common::{Id, format_error_chain};
use moon_config::{CacheConfig, RemoteConfig};
use moon_hash::Digest;
use rustc_hash::FxHashMap;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::{Duration, SystemTime};
use tokio::task::{AbortHandle, JoinHandle, JoinSet};
use tracing::{debug, warn};

/// Upper bound on how long shutdown waits for queued background cache writes
/// (remote uploads, etc.) to drain. A hung backend must never make exiting
/// slower than just running the task would have been; stragglers past this are
/// aborted and reported, and simply get re-uploaded on the next run.
const BACKGROUND_FLUSH_TIMEOUT: Duration = Duration::from_secs(60);

#[derive(Clone, Debug)]
pub struct CacheContext {
    pub cache_dir: PathBuf,
    pub cache_config: Arc<CacheConfig>,
    pub config_dir: PathBuf,
    pub remote_config: Arc<RemoteConfig>,
    pub remote_debug: bool,
    pub workspace_root: PathBuf,
}

#[derive(Clone, Debug)]
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

#[derive(Debug)]
pub struct Storage {
    background_tasks: Arc<Mutex<Vec<JoinHandle<miette::Result<()>>>>>,
    local_backends: Vec<BoxedStorageBackend>,
    remote_backends: Vec<BoxedStorageBackend>,

    context: CacheContext,
    options: StorageOptions,
}

impl Storage {
    pub fn new(context: CacheContext) -> Self {
        Self {
            background_tasks: Arc::new(Mutex::new(vec![])),
            local_backends: vec![],
            remote_backends: vec![],
            context,
            options: StorageOptions::default(),
        }
    }

    pub fn with_options(&self, options: StorageOptions) -> Self {
        Self {
            background_tasks: Arc::clone(&self.background_tasks),
            local_backends: self.local_backends.clone(),
            remote_backends: self.remote_backends.clone(),
            context: self.context.clone(),
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
                        error = format_error_chain(&error),
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
        self.get_backends_with_options(&self.options)
    }

    pub fn get_backends_with_options(&self, options: &StorageOptions) -> Vec<&BoxedStorageBackend> {
        let mut backends = vec![];

        if options.include_local {
            backends.extend(self.local_backends.iter());
        }

        if options.include_remote {
            backends.extend(self.remote_backends.iter());
        }

        if !options.only_backends.is_empty() {
            backends.retain(|backend| options.only_backends.contains(backend.get_id()));
        }

        backends
    }

    pub fn get_local_backends(&self) -> Vec<&BoxedStorageBackend> {
        self.get_backends_with_options(&StorageOptions {
            // Respect previously configured options
            include_remote: false,
            ..self.options.clone()
        })
    }

    pub fn get_remote_backends(&self) -> Vec<&BoxedStorageBackend> {
        self.get_backends_with_options(&StorageOptions {
            // Respect previously configured options
            include_local: false,
            ..self.options.clone()
        })
    }

    pub fn is_local_enabled(&self) -> bool {
        !self.local_backends.is_empty()
    }

    pub fn is_remote_enabled(&self) -> bool {
        !self.remote_backends.is_empty()
    }

    /// Garbage-collect the writable local backends. Remotes are skipped — they
    /// manage their own eviction server-side. A failure in one backend is logged
    /// and skipped rather than aborting the whole clean.
    pub async fn clean(&self, lifetime: Duration) -> miette::Result<BlobCleanStats> {
        let mut stats = BlobCleanStats::default();

        for backend in &self.local_backends {
            if !backend.is_writable() {
                continue;
            }

            match backend.gc(lifetime).await {
                Ok(backend_stats) => {
                    stats.blobs_removed += backend_stats.blobs_removed;
                    stats.bytes_saved += backend_stats.bytes_saved;
                }
                Err(error) => {
                    warn!(
                        storage = backend.get_id().as_str(),
                        error = format_error_chain(&error),
                        "Failed to garbage collect storage backend"
                    );
                }
            }
        }

        Ok(stats)
    }

    pub async fn retrieve_blob(&self, digest: Digest) -> miette::Result<Option<BlobOutput>> {
        let mut results = self.retrieve_blobs(vec![digest]).await?;

        if !results.is_empty() {
            return Ok(Some(results.remove(0)));
        }

        Ok(None)
    }

    pub async fn retrieve_blobs(&self, digests: Vec<Digest>) -> miette::Result<Vec<BlobOutput>> {
        for backend in self.get_backends() {
            if !backend.is_readable() {
                continue;
            }

            let result = Arc::clone(backend)
                .retrieve_blobs_batched(Digest::default(), digests.clone())
                .await?;

            if result.blobs.len() > 0 {
                return Ok(result.blobs);
            }
        }

        Ok(vec![])
    }

    pub async fn store_blob(&self, blob: BlobInput) -> miette::Result<()> {
        self.store_blobs(vec![blob]).await
    }

    pub async fn store_blobs(&self, blobs: Vec<BlobInput>) -> miette::Result<()> {
        let mut background_tasks = self.background_tasks.lock().unwrap();

        for backend in self.get_backends() {
            if !backend.is_writable() {
                continue;
            }

            let backend = Arc::clone(backend);
            let blobs = blobs.clone();

            background_tasks.push(tokio::spawn(Box::pin(async move {
                backend
                    .store_blobs_batched(Digest::default(), blobs)
                    .await
                    .map(|_| ())
            })));
        }

        Ok(())
    }

    pub async fn load_manifest(&self, digest: &Digest) -> miette::Result<Option<ManifestSource>> {
        debug!(hash = digest.hash.as_str(), "Checking for a cache manifest");

        for backend in self.get_backends() {
            if !backend.is_readable() {
                continue;
            }

            if let Some(manifest) = backend.retrieve_manifest(digest.to_owned()).await? {
                debug!(
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

        debug!(hash = digest.hash.as_str(), "Cache miss on manifest");

        Ok(None)
    }

    pub async fn archive_manifest(
        &self,
        digest: &Digest,
        manifest: Manifest,
        action_blob: Option<BlobInput>,
    ) -> miette::Result<()> {
        let mut background_tasks = self.background_tasks.lock().unwrap();

        debug!(
            hash = digest.hash.as_str(),
            files = manifest.files.len(),
            symlinks = manifest.symlinks.len(),
            exit_code = manifest.exit_code,
            "Archiving cache manifest"
        );

        // Store the manifest in all backends in parallel, but if any fail,
        // continue storing the rest for failover/redundancy in the future
        for backend in self.get_backends() {
            if !backend.is_writable() {
                continue;
            }

            background_tasks.push(tokio::spawn(Box::pin(persist_manifest_in_backend(
                Arc::clone(backend),
                digest.to_owned(),
                manifest.clone(),
                self.context.workspace_root.clone(),
                action_blob.clone(),
            ))));
        }

        debug!(
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
            remote,
        } = manifest_source;
        let mut backends = VecDeque::from_iter(self.get_backends());
        let mut count = 1;

        debug!(hash = digest.hash.as_str(), "Hydrating cache manifest");

        // Hydrate the manifest from the backend it was originally loaded from,
        // as that's the most likely to have all the blobs available
        hydrate_manifest_from_backend(&original_backend, digest, &mut manifest).await?;

        // If the original backend doesn't have all the blobs available,
        // we should attempt to hydrate from the other backends,
        // and also copy the missing blobs to the original backend
        while !manifest.is_hydrated()
            && let Some(backend) = backends.pop_front()
        {
            if !backend.is_readable() || backend.get_id() == original_backend.get_id() {
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
            debug!(
                hash = digest.hash.as_str(),
                "Hydrated cache manifest from {count} storage backends"
            );

            // A remote hit leaves the local tier cold. Warm it from the
            // now-in-memory blobs so the next run resolves locally instead of
            // round-tripping to the remote again
            if remote {
                self.warm_local_backends(digest, &manifest).await;
            }

            return Ok(Some(manifest));
        }

        debug!(
            hash = digest.hash.as_str(),
            "Failed to hydrate cache manifest as some blobs were missing"
        );

        Ok(None)
    }

    /// Warm the local tier after a remote cache hit by persisting the fully
    /// hydrated manifest and its blobs into every active, writable local
    /// backend, so the next run resolves locally instead of round-tripping to
    /// the remote.
    async fn warm_local_backends(&self, digest: &Digest, manifest: &Manifest) {
        let mut background_tasks = self.background_tasks.lock().unwrap();

        for backend in self.get_local_backends() {
            if !backend.is_writable() {
                continue;
            }

            debug!(
                storage = backend.get_id().as_str(),
                hash = digest.hash.as_str(),
                "Warming local storage backend from remote cache hit"
            );

            // No action blob here: warming targets local backends only, which
            // don't validate the RE contract, so the fingerprint file needn't be
            // re-stored into the local CAS.
            background_tasks.push(tokio::spawn(Box::pin(persist_manifest_in_backend(
                Arc::clone(backend),
                digest.to_owned(),
                manifest.clone(),
                self.context.workspace_root.clone(),
                None,
            ))));
        }
    }

    pub async fn wait_for_background_tasks(&self) -> miette::Result<()> {
        let background_tasks = {
            self.background_tasks
                .lock()
                .unwrap()
                .drain(0..)
                .collect::<Vec<_>>()
        };

        if background_tasks.is_empty() {
            return Ok(());
        }

        debug!(
            timeout_secs = BACKGROUND_FLUSH_TIMEOUT.as_secs(),
            tasks = background_tasks.len(),
            "Waiting for background storage tasks to complete"
        );

        // Keep abort handles so stragglers can be cancelled if the drain times
        // out (awaiting the handles themselves would consume them first).
        let abort_handles = background_tasks
            .iter()
            .map(JoinHandle::abort_handle)
            .collect::<Vec<AbortHandle>>();

        let drained = tokio::time::timeout(BACKGROUND_FLUSH_TIMEOUT, async {
            for handle in background_tasks {
                // These are best-effort cache writes; a failed or panicked
                // upload must not fail shutdown, so swallow the result.
                let _ = handle.await;
            }
        })
        .await;

        if drained.is_err() {
            let dropped = abort_handles
                .iter()
                .filter(|handle| !handle.is_finished())
                .count();

            for handle in abort_handles {
                handle.abort();
            }

            warn!(
                timeout_secs = BACKGROUND_FLUSH_TIMEOUT.as_secs(),
                dropped, "Timed out flushing background storage tasks; {dropped} were dropped",
            );
        }

        Ok(())
    }
}

async fn persist_manifest_in_backend(
    backend: BoxedStorageBackend,
    digest: Digest,
    mut manifest: Manifest,
    workspace_root: PathBuf,
    action_blob: Option<BlobInput>,
) -> miette::Result<()> {
    let mut blob_inputs = manifest.collect_blob_inputs(&workspace_root);

    // The action digest addresses the hash manifest that produced it, so that
    // file *is* the blob the digest names. The task runner (which owns the cache
    // layout) supplies it, and it's uploaded with the outputs: backends that
    // validate the RE contract reject an action result whose action digest is
    // absent from the CAS ("action digest <hash>/<size> not found in CAS"),
    // because a client is expected to have uploaded it before referencing it.
    blob_inputs.extend(action_blob);

    // Before we store the manifest, we should ensure all associated blobs are stored.
    // This ensures we don't end up with dangling manifests that reference missing blobs.
    if !blob_inputs.is_empty() {
        manifest.upload_started_at = Some(SystemTime::now());

        let stored = Arc::clone(&backend)
            .store_blobs_batched(digest.clone(), blob_inputs)
            .await?;

        manifest.upload_completed_at = Some(SystemTime::now());

        if !stored.success {
            return Ok(());
        }
    }

    if backend.get_capabilities().store_manifests {
        if let Err(error) = backend.store_manifest(digest.clone(), manifest).await {
            warn!(
                storage = backend.get_id().as_str(),
                hash = digest.hash.as_str(),
                error = format_error_chain(&error),
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
) -> miette::Result<FxHashMap<Digest, BlobContent>> {
    let blob_digests = manifest.collect_unhydrated_blob_digests();

    // Retrieve all blobs for digests that have yet to be hydrated
    let received = Arc::clone(backend)
        .retrieve_blobs_batched(digest.clone(), blob_digests)
        .await?;

    let blobs_map = received
        .blobs
        .into_iter()
        .map(|blob| (blob.digest, blob.content))
        .collect::<FxHashMap<_, _>>();

    // And then copy their data into the manifest
    manifest.hydrate(&blobs_map)?;

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
        if let Some(content) = blobs_map.get(&digest) {
            blob_inputs.push(BlobInput {
                content: content.to_owned(),
                digest,
            });
        }
    }

    // Then store them in the original backend in which they were missing
    if !blob_inputs.is_empty() && original_backend.is_writable() {
        debug!(
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
