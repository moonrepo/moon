use moon_cache::CacheEngine;
use moon_common::color;
use moon_task::Task;
use starbase_archive::tar::TarUnpacker;
use starbase_archive::Archiver;
use starbase_utils::fs;
use std::path::Path;
use tracing::warn;

pub enum HydrateFrom {
    LocalCache,
    PreviousOutput,
    RemoteCache,
}

pub struct OutputHydrater<'task> {
    task: &'task Task,
    workspace_root: &'task Path,
}

impl<'task> OutputHydrater<'task> {
    pub fn unpack_archive(&self, hash: &str, cache_engine: &CacheEngine) -> miette::Result<bool> {
        let archive_file = cache_engine.hash.get_archive_path(hash);

        // If cache disabled or archive doesn't exist, do nothing
        if !cache_engine.get_mode().is_readable() || !archive_file.exists() {
            return Ok(false);
        }

        // Create the archiver instance based on task outputs
        let mut archive = Archiver::new(&self.workspace_root, &archive_file);

        for output_file in &self.task.output_files {
            archive.add_source_file(output_file.as_str(), None);
        }

        for output_glob in &self.task.output_globs {
            archive.add_source_glob(output_glob.as_str());
        }

        // Unpack the archive and handle log files
        let state_dir = cache_engine.state.get_target_dir(&self.task.target);
        let stdout_log = self.workspace_root.join("stdout.log");
        let stderr_log = self.workspace_root.join("stderr.log");

        match archive.unpack(TarUnpacker::new_gz) {
            Ok(_) => {
                // Old archives place the log files in the root of the
                // archive/workspace, so move them manually
                if stdout_log.exists() {
                    fs::rename(stdout_log, state_dir.join("stdout.log"))?;
                }

                if stderr_log.exists() {
                    fs::rename(stderr_log, state_dir.join("stderr.log"))?;
                }
            }
            Err(error) => {
                warn!(
                    target = self.task.target.as_str(),
                    "Failed to hydrate {} outputs from cache: {}",
                    color::label(&self.task.target),
                    color::muted_light(error.to_string()),
                );

                // Delete target outputs to ensure a clean slate
                for output in &self.task.output_files {
                    fs::remove_file(output.to_logical_path(&self.workspace_root));
                }

                // And delete workspace root log files
                fs::remove_file(stdout_log)?;
                fs::remove_file(stderr_log)?;
            }
        }

        Ok(true)
    }
}
