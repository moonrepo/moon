use crate::run_state::TaskRunState;
use crate::task_runner_error::TaskRunnerError;
use miette::IntoDiagnostic;
use moon_app_context::AppContext;
use moon_blob::Blob;
use moon_cache::{StorageOptions, TaskManifest};
use moon_common::color;
use moon_common::path::PathExt;
use moon_common::path::to_relative_virtual_string;
use moon_daemon_client::DaemonClient;
use moon_hash::ContentHash;
use moon_manifest::{ManifestError, TaskManifestFile as ManifestFile, TaskManifestPacker};
use moon_task::Task;
use starbase_archive::Archiver;
use starbase_utils::fs;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::task::spawn_blocking;
use tracing::{debug, instrument, warn};

type Manifest = TaskManifest;
type ManifestPacker = TaskManifestPacker;

pub enum ArchiveOutcome {
    Skipped,
    Queued,
}

pub(crate) struct StagedLocalArchive {
    pub scratch: Arc<TempDir>,
    pub manifest: Manifest,
    pub declared_dirs: Vec<PathBuf>,
    pub log_paths: Vec<PathBuf>,
    pub has_required_outputs: bool,
}

/// Cache outputs to the `.moon/cache` folder and to cloud storage,
/// so that subsequent builds are faster, and any local outputs
/// can be hydrated easily.
pub struct OutputArchiver<'task> {
    app_context: &'task Arc<AppContext>,
    task: &'task Arc<Task>,
    daemon_client: Option<DaemonClient>,
}

impl OutputArchiver<'_> {
    pub fn new<'task>(
        app_context: &'task Arc<AppContext>,
        task: &'task Arc<Task>,
        daemon_client: Option<DaemonClient>,
    ) -> miette::Result<OutputArchiver<'task>> {
        Ok(OutputArchiver {
            task,
            app_context,
            daemon_client,
        })
    }

    #[instrument(skip(self, state))]
    pub async fn archive(
        &self,
        hash: &str,
        state: &TaskRunState,
    ) -> miette::Result<ArchiveOutcome> {
        // Check that outputs actually exist
        if self.task.is_build_type() && !self.has_outputs_been_created()? {
            return Err(TaskRunnerError::MissingOutputs {
                target: self.task.target.clone(),
            }
            .into());
        }

        let task_target = self.task.target.as_str();

        if state.local_cache_writable && state.remote_cache_writable {
            debug!(
                task_target,
                hash, "Storing task outputs in local and remote caches"
            );
        } else if state.local_cache_writable {
            debug!(task_target, hash, "Storing task outputs in local cache");
        } else if state.remote_cache_writable {
            debug!(task_target, hash, "Storing task outputs in remote cache");
        } else {
            debug!(
                task_target,
                hash, "Cache is not writable, skipping task output archiving"
            );

            return Ok(ArchiveOutcome::Skipped);
        }

        let use_local = state.local_cas_enabled && state.local_cache_writable;
        let use_remote = state.remote_cache_writable;

        // Store the manifest in the local/remote caches
        if use_local || use_remote {
            let manifest = self.create_cache_manifest(state).await?;

            if let Some(mut daemon) = self.daemon_client.clone() {
                daemon
                    .archive_task_outputs(
                        self.task.target.to_string(),
                        state.digest.clone(),
                        manifest,
                        use_local,
                        use_remote,
                    )
                    .await?;
            } else {
                self.app_context
                    .cache_engine
                    .storage
                    .with_options(StorageOptions {
                        include_local: use_local,
                        include_remote: use_remote,
                        ..Default::default()
                    })
                    .archive_task_manifest(&state.digest, manifest)
                    .await?;
            }
        }

        // Create the archive file (legacy / temporary)
        if !state.local_cas_enabled && state.local_cache_writable {
            self.pack_local_archive(hash, state).await?;
        }

        Ok(ArchiveOutcome::Queued)
    }

    #[instrument(skip(self))]
    pub fn has_outputs_been_created(&self) -> miette::Result<bool> {
        Self::has_outputs_at(self.task, &self.app_context.workspace_root)
    }

    #[instrument(skip(self))]
    pub fn has_previous_outputs_been_created(
        &self,
        previous_globs_hash: &str,
    ) -> miette::Result<bool> {
        if !self.has_output_files_been_created() {
            return Ok(false);
        }

        if let Some(globs_hash) = self.hash_output_globs()?
            && globs_hash != previous_globs_hash
        {
            debug!(
                task_target = self.task.target.as_str(),
                "Files matched by output globs differ from the previous run, will hydrate"
            );
            return Ok(false);
        }
        Ok(true)
    }

    fn has_outputs_at(task: &Task, root: &Path) -> miette::Result<bool> {
        for (output, params) in &task.output_files {
            if !output.to_path(root).exists() && !params.optional {
                return Ok(false);
            }
        }
        if task
            .output_globs
            .keys()
            .any(|glob| !glob.as_str().starts_with('!'))
        {
            let outputs = task.get_output_files(root, false)?;
            if outputs.is_empty()
                && !task
                    .outputs
                    .iter()
                    .all(|output| output.is_glob() && output.is_optional())
            {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn validate_staged_sources(manifest: &Manifest, scratch: &TempDir) -> miette::Result<()> {
        let root = std::fs::canonicalize(scratch.path()).into_diagnostic()?;
        for source in manifest
            .files
            .iter()
            .filter_map(|file| file.source_path.as_ref())
        {
            if !std::fs::canonicalize(source)
                .into_diagnostic()?
                .starts_with(&root)
            {
                return Err(miette::miette!(
                    "Cached output escapes private promotion directory"
                ));
            }
        }
        for link in &manifest.symlinks {
            let target = link.target.to_logical_path(&root);
            if !std::fs::canonicalize(target)
                .into_diagnostic()?
                .starts_with(&root)
            {
                return Err(miette::miette!(
                    "Cached symlink escapes private promotion directory"
                ));
            }
        }
        Ok(())
    }

    /// Validate a complete legacy archive before either promotion or hydration
    /// can observe its content. The returned source paths stay alive with scratch.
    pub(crate) async fn stage_local_archive(
        app_context: &Arc<AppContext>,
        task: &Arc<Task>,
        archive_path: PathBuf,
    ) -> miette::Result<Option<StagedLocalArchive>> {
        let cache = &app_context.cache_engine;
        let logs = cache
            .state
            .get_target_dir(&task.target)
            .relative_to(&app_context.workspace_root)
            .into_diagnostic()?;
        fs::create_dir_all(&cache.temp_dir)?;
        let mut scratch_builder = tempfile::Builder::new();
        scratch_builder.prefix("hydrate-");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            scratch_builder.permissions(std::fs::Permissions::from_mode(0o700));
        }
        let scratch = Arc::new(
            scratch_builder
                .tempdir_in(&cache.temp_dir)
                .into_diagnostic()?,
        );
        let task = Arc::clone(task);
        let staging = Arc::clone(&scratch);
        spawn_blocking(move || {
            if let Err(error) = Archiver::new(staging.path(), &archive_path).unpack_from_ext() {
                warn!(
                    task_target = task.target.as_str(),
                    archive_file = ?archive_path,
                    "Failed to stage cached task outputs: {}",
                    color::muted_light(error.to_string()),
                );
                return Ok(None);
            }
            let mut packer = ManifestPacker::new(staging.path().to_path_buf());
            for output in task.get_output_files(staging.path(), true)? {
                if let Err(error) = packer.inherit_output(output) {
                    if matches!(
                        error.downcast_ref::<ManifestError>(),
                        Some(ManifestError::OutputSymlinkOutsideOfWorkspace { .. })
                    ) {
                        debug!(
                            task_target = task.target.as_str(),
                            "Skipping cache archive with an unsupported symlink output"
                        );
                        return Ok(None);
                    }
                    return Err(error);
                }
            }
            let mut manifest = packer.pack();
            let mut pending = task
                .output_files
                .keys()
                .map(|path| path.to_logical_path(staging.path()))
                .filter(|path| std::fs::symlink_metadata(path).is_ok_and(|meta| meta.is_dir()))
                .collect::<Vec<_>>();
            let mut declared_dirs = Vec::new();
            while let Some(dir) = pending.pop() {
                if declared_dirs.contains(&dir) {
                    continue;
                }
                for entry in std::fs::read_dir(&dir).into_diagnostic()? {
                    let entry = entry.into_diagnostic()?;
                    if entry.file_type().into_diagnostic()?.is_dir() {
                        pending.push(entry.path());
                    }
                }
                declared_dirs.push(dir);
            }
            let log_dir = logs.to_logical_path(staging.path());
            let log_paths = [log_dir.join("stdout.log"), log_dir.join("stderr.log")]
                .into_iter()
                .filter(|path| std::fs::symlink_metadata(path).is_ok_and(|meta| meta.is_file()))
                .collect::<Vec<_>>();
            for path in &log_paths {
                let blob = Blob::from_file(path)?;
                if path.ends_with("stdout.log") {
                    manifest.stdout_digest = Some(blob.digest);
                    manifest.stdout_bytes = Some(blob.bytes);
                } else {
                    manifest.stderr_digest = Some(blob.digest);
                    manifest.stderr_bytes = Some(blob.bytes);
                }
            }
            Self::validate_staged_sources(&manifest, &staging)?;
            let has_required_outputs = Self::has_outputs_at(&task, staging.path())?;
            Ok::<_, miette::Report>(Some(StagedLocalArchive {
                scratch: staging,
                manifest,
                declared_dirs,
                log_paths,
                has_required_outputs,
            }))
        })
        .await
        .into_diagnostic()?
    }

    pub(crate) fn validate_staged_entries(
        manifest: &Manifest,
        declared_dirs: &[PathBuf],
        log_paths: &[PathBuf],
        root: &Path,
    ) -> miette::Result<()> {
        let mut allowed = manifest
            .files
            .iter()
            .filter_map(|file| file.source_path.clone())
            .collect::<Vec<_>>();
        allowed.extend(
            manifest
                .symlinks
                .iter()
                .map(|link| link.path.to_logical_path(root)),
        );
        allowed.extend(declared_dirs.iter().cloned());
        allowed.extend(log_paths.iter().cloned());
        let mut pending = vec![root.to_path_buf()];
        while let Some(dir) = pending.pop() {
            for entry in std::fs::read_dir(&dir).into_diagnostic()? {
                let entry = entry.into_diagnostic()?;
                let path = entry.path();
                let kind = entry.file_type().into_diagnostic()?;
                if kind.is_dir() {
                    if !allowed.iter().any(|file| file.starts_with(&path)) {
                        return Err(miette::miette!(
                            "Cached archive contains undeclared directory {}",
                            path.display()
                        ));
                    }
                    pending.push(path);
                } else if !allowed.contains(&path) {
                    return Err(miette::miette!(
                        "Cached archive contains undeclared entry {}",
                        path.display()
                    ));
                }
            }
        }
        Ok(())
    }
    /// Create a hash from the workspace relative paths of all files currently
    /// matched by output globs, so that a subsequent run can verify that the
    /// exact same set of files still exists. Returns `None` if the task has
    /// no globs that can match files (negated globs are ignored).
    #[instrument(skip(self))]
    pub fn hash_output_globs(&self) -> miette::Result<Option<String>> {
        if !self.has_output_globs() {
            return Ok(None);
        }

        let workspace_root = &self.app_context.workspace_root;
        let mut paths = vec![];

        for file in self.task.get_output_files(workspace_root, false)? {
            paths.push(to_relative_virtual_string(file, workspace_root)?);
        }

        // Walk order is not guaranteed
        paths.sort();

        Ok(Some(ContentHash::hash_bytes(paths.join("\0"))?.to_string()))
    }

    fn has_output_files_been_created(&self) -> bool {
        self.task.output_files.iter().all(|(output, params)| {
            params.optional || output.to_path(&self.app_context.workspace_root).exists()
        })
    }

    /// Return true if the task has output globs that can match files,
    /// as negated globs cannot match anything on their own.
    fn has_output_globs(&self) -> bool {
        self.task
            .output_globs
            .keys()
            .any(|glob| !glob.as_str().starts_with('!'))
    }

    #[instrument(skip(self, state))]
    async fn create_cache_manifest(&self, state: &TaskRunState) -> miette::Result<TaskManifest> {
        let task = Arc::clone(self.task);
        let workspace_root = self.app_context.workspace_root.clone();

        // Building the manifest incurs a lot of file system calls,
        // so we run it in a blocking thread to avoid blocking the async runtime
        let mut packer = spawn_blocking(move || {
            let outputs = task.get_output_files(&workspace_root, true)?;
            let mut packer = TaskManifestPacker::new(workspace_root);

            for output in outputs {
                packer.inherit_output(output)?;
            }

            Ok::<_, miette::Report>(packer)
        })
        .await
        .into_diagnostic()??;

        // Then inherit the execution operation metadata
        packer.inherit_operation(&state.operation)?;

        // Then inherit the source fingerprint, which lives in the local CAS.
        // Without a local tier there's nothing to attach, and the manifest is
        // archived without it.
        if let Some(blob) = self
            .app_context
            .cache_engine
            .storage
            .load_hash_manifest(&state.digest)
            .await?
        {
            packer.inherit_source(&state.digest, blob)?;
        }

        Ok(packer.pack())
    }

    #[instrument(skip(self, state))]
    async fn pack_local_archive(&self, hash: &str, state: &TaskRunState) -> miette::Result<()> {
        let archive_file = self.app_context.cache_engine.hash.get_archive_path(hash);

        if state.local_cache_writable && archive_file.exists() {
            debug!(
                task_target = self.task.target.as_str(),
                hash, "Skipping archive, task outputs already persisted"
            );
        } else if !state.local_cache_writable {
            debug!(
                task_target = self.task.target.as_str(),
                hash, "Cache is not writable, skipping output archiving"
            );

            return Ok(());
        }

        debug!(
            task_target = self.task.target.as_str(),
            hash,
            archive_file = ?archive_file, "Archiving task outputs from project"
        );

        // Clone values to run in a blocking thread
        let app_context = Arc::clone(self.app_context);
        let task = Arc::clone(self.task);
        let hash = hash.to_string();

        spawn_blocking(move || {
            // Create the archiver instance based on task outputs
            let mut archive = Archiver::new(&app_context.workspace_root, &archive_file);

            for output_file in task.output_files.keys() {
                archive.add_source_file(output_file.as_str(), None);
            }

            for output_glob in task.output_globs.keys() {
                archive.add_source_glob(output_glob.as_str());
            }

            // Also include stdout/stderr logs in the tarball
            let state_dir = app_context.cache_engine.state.get_target_dir(&task.target);

            archive.add_source_file(state_dir.join("stdout.log"), None);

            archive.add_source_file(state_dir.join("stderr.log"), None);

            // Pack the archive
            if let Err(error) = archive.pack_from_ext() {
                warn!(
                    task_target = task.target.as_str(),
                    hash,
                    archive_file = ?archive_file,
                    "Failed to package task outputs into archive: {}",
                    color::muted_light(error.to_string()),
                );

                return false;
            }

            true
        })
        .await
        .into_diagnostic()?;

        Ok(())
    }
}
