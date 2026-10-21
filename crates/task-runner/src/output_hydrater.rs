use crate::output_archiver::OutputArchiver;
use crate::run_state::TaskRunState;
use crate::task_runner_error::TaskRunnerError;
use miette::IntoDiagnostic;
use moon_app_context::AppContext;
use moon_cache::{StorageOptions, TaskManifest, TaskManifestSource, TaskManifestUnpacker};
use moon_common::path::WorkspaceRelativePath;
use moon_daemon_client::DaemonClient;
use moon_task::Task;
use starbase_utils::{fs, glob::GlobSet};
use std::fmt::{self, Debug};
use std::io::ErrorKind;
use std::sync::Arc;
use tracing::{debug, instrument};

pub enum HydrateFrom {
    PreviousOutput,
    LocalArchive,
    Storage(Box<TaskManifestSource>),
}

impl Debug for HydrateFrom {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HydrateFrom::PreviousOutput => write!(f, "PreviousOutput"),
            HydrateFrom::LocalArchive => write!(f, "LocalArchive"),
            HydrateFrom::Storage(source) => write!(f, "Storage({})", source.backend.get_id()),
        }
    }
}

pub enum HydrateOutcome {
    Skipped,
    Missed,
    Hit,
    // Boxed to keep the enum small: `TaskManifest` dwarfs the unit variants, so
    // every `HydrateOutcome` would otherwise be sized for this one case.
    HitFromStorage(Box<TaskManifest>, bool),
}

pub struct OutputHydrater<'task> {
    app_context: &'task Arc<AppContext>,
    task: &'task Arc<Task>,
    task_output_globset: GlobSet<'static>,
    daemon_client: Option<DaemonClient>,
}

impl OutputHydrater<'_> {
    pub fn new<'task>(
        app_context: &'task Arc<AppContext>,
        task: &'task Arc<Task>,
        daemon_client: Option<DaemonClient>,
    ) -> miette::Result<OutputHydrater<'task>> {
        Ok(OutputHydrater {
            task_output_globset: GlobSet::new_owned(task.output_globs.keys())?,
            task,
            app_context,
            daemon_client,
        })
    }

    #[instrument(skip(self, state))]
    pub async fn hydrate(
        &self,
        from: HydrateFrom,
        hash: &str,
        state: &TaskRunState,
    ) -> miette::Result<HydrateOutcome> {
        match from {
            HydrateFrom::PreviousOutput => Ok(HydrateOutcome::Hit),

            HydrateFrom::LocalArchive => self.unpack_local_archive(hash, state).await,

            HydrateFrom::Storage(source) => {
                if !source.remote && !state.local_cas_enabled {
                    return self.unpack_local_archive(hash, state).await;
                }

                let task_target = self.task.target.as_str();

                if state.local_cache_readable && state.remote_cache_readable {
                    debug!(
                        task_target,
                        hash, "Hydrating task outputs from local and remote caches"
                    );
                } else if state.local_cache_readable {
                    debug!(task_target, hash, "Hydrating task outputs from local cache");
                } else if state.remote_cache_readable {
                    debug!(
                        task_target,
                        hash, "Hydrating task outputs from remote cache"
                    );
                } else {
                    debug!(
                        task_target,
                        hash, "Cache is not readable, skipping task output hydration"
                    );

                    return Ok(HydrateOutcome::Skipped);
                }

                let use_local = state.local_cas_enabled && state.local_cache_readable;
                let use_remote = state.remote_cache_readable;
                let is_remote_backend = source.remote;

                // Validate the output paths are legit before doing anything
                self.validate_output_paths(&source.manifest)?;

                // Retrieve the manifest from the local/remote caches
                let mut manifest = None;

                if let Some(mut daemon) = self.daemon_client.clone() {
                    let res = daemon
                        .hydrate_task_outputs(
                            self.task.target.to_string(),
                            state.digest.clone(),
                            source.manifest,
                            use_local,
                            use_remote,
                            source.backend.get_id().to_string(),
                        )
                        .await?;

                    if res.hydrated
                        && let Some(action_result) = res.manifest
                    {
                        manifest = Some(TaskManifest::from_bazel_action_result(action_result)?);
                    }
                } else {
                    manifest = self
                        .app_context
                        .cache_engine
                        .storage
                        .with_options(StorageOptions {
                            include_local: use_local,
                            include_remote: use_remote,
                            ..Default::default()
                        })
                        .hydrate_task_manifest(&state.digest, *source)
                        .await?;

                    if let Some(manifest) = &manifest {
                        // Delete existing outputs only after all cache reads succeed.
                        // A failed read is a cache miss and must not remove outputs
                        // that the task may rely on recreating.
                        self.delete_existing_outputs()?;
                        TaskManifestUnpacker::new(
                            manifest,
                            self.app_context.workspace_root.clone(),
                        )
                        .unpack()?;
                    }
                }

                Ok(match manifest {
                    Some(manifest) => {
                        HydrateOutcome::HitFromStorage(Box::new(manifest), is_remote_backend)
                    }
                    None => HydrateOutcome::Missed,
                })
            }
        }
    }

    #[instrument(skip(self, state))]
    async fn unpack_local_archive(
        &self,
        hash: &str,
        state: &TaskRunState,
    ) -> miette::Result<HydrateOutcome> {
        let archive_file = self.app_context.cache_engine.hash.get_archive_path(hash);
        let task_target = self.task.target.as_str();

        if state.local_cache_readable && archive_file.exists() {
            debug!(
                task_target,
                hash,
                archive_file = ?archive_file,
                "Hydrating task outputs from local cache archive (legacy)"
            );
        } else if !state.local_cache_readable || !archive_file.exists() {
            debug!(
                task_target,
                hash, "Cache is not readable, skipping output hydration"
            );

            return Ok(HydrateOutcome::Skipped);
        }

        let Some(staged) =
            OutputArchiver::stage_local_archive(self.app_context, self.task, archive_file).await?
        else {
            return Ok(HydrateOutcome::Missed);
        };

        OutputArchiver::validate_staged_entries(
            &staged.manifest,
            &staged.declared_dirs,
            &staged.log_paths,
            staged.scratch.path(),
        )?;
        self.validate_output_paths(&staged.manifest)?;
        if !staged.has_required_outputs {
            return Ok(HydrateOutcome::Missed);
        }
        // Nothing in the live workspace is removed until extraction and all
        // declared-path checks have succeeded. A failure below must never hit.
        self.delete_existing_outputs()?;
        for dir in &staged.declared_dirs {
            let relative = dir.strip_prefix(staged.scratch.path()).into_diagnostic()?;
            fs::create_dir_all(self.app_context.workspace_root.join(relative))?;
        }
        TaskManifestUnpacker::new(&staged.manifest, self.app_context.workspace_root.clone())
            .unpack()?;
        for log in staged.log_paths {
            let relative = log.strip_prefix(staged.scratch.path()).into_diagnostic()?;
            let destination = self.app_context.workspace_root.join(relative);
            if let Some(parent) = destination.parent() {
                fs::create_dir_all(parent)?;
            }
            std::fs::copy(log, destination).into_diagnostic()?;
        }
        Ok(HydrateOutcome::Hit)
    }

    fn delete_existing_outputs(&self) -> miette::Result<()> {
        for output in self
            .task
            .get_output_files(&self.app_context.workspace_root, true)?
        {
            match std::fs::symlink_metadata(&output) {
                Ok(_) => fs::remove(output)?,
                Err(error) if error.kind() == ErrorKind::NotFound => {}
                Err(error) => return Err(error).into_diagnostic(),
            }
        }

        Ok(())
    }

    pub(crate) fn validate_output_paths(&self, manifest: &TaskManifest) -> miette::Result<()> {
        for file in &manifest.files {
            if file.digest.is_some() {
                self.validate_output_path(&file.path)?;
            }
        }

        for link in &manifest.symlinks {
            self.validate_output_path(&link.path)?;
        }

        Ok(())
    }

    fn validate_output_path(&self, rel_path: &WorkspaceRelativePath) -> miette::Result<()> {
        if self.task.output_files.contains_key(rel_path) {
            return Ok(());
        }

        for declared_output in self.task.output_files.keys() {
            if rel_path.starts_with(declared_output) {
                return Ok(());
            }
        }

        if !self.task.output_globs.is_empty() && self.task_output_globset.matches(rel_path.as_str())
        {
            return Ok(());
        }

        Err(TaskRunnerError::OutputFileNotDeclared {
            target: self.task.target.clone(),
            output: rel_path.to_logical_path(&self.app_context.workspace_root),
        }
        .into())
    }
}
