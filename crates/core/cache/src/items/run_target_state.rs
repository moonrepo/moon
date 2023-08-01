use crate::cache_item;
use crate::helpers::get_cache_mode;
use moon_common::path::WorkspaceRelativePathBuf;
use moon_logger::{map_list, trace, warn};
use serde::{Deserialize, Serialize};
use starbase_archive::tar::{TarPacker, TarUnpacker};
use starbase_archive::Archiver;
use starbase_styles::color;
use starbase_utils::{fs, glob, json};
use std::path::{Path, PathBuf};

#[derive(Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct RunTargetState {
    pub exit_code: i32,

    pub hash: String,

    pub last_run_time: u128,

    pub target: String,

    #[serde(skip)]
    pub path: PathBuf,
}

cache_item!(RunTargetState);

fn create_archive<'o>(
    workspace_root: &'o Path,
    archive_file: &'o Path,
    output_paths: &[WorkspaceRelativePathBuf],
) -> Archiver<'o> {
    let mut archive = Archiver::new(workspace_root, archive_file);

    // Outputs are relative from the workspace root
    if !output_paths.is_empty() {
        for output in output_paths {
            if glob::is_glob(output) {
                archive.add_source_glob(output.as_str(), None);
            } else {
                archive.add_source_file(output.as_str(), None);
            }
        }
    }

    archive
}

impl RunTargetState {
    pub fn archive_outputs(
        &self,
        archive_file: &Path,
        workspace_root: &Path,
        output_paths: &[WorkspaceRelativePathBuf],
    ) -> miette::Result<bool> {
        if get_cache_mode().is_writable() && !archive_file.exists() {
            let mut archive = create_archive(workspace_root, archive_file, output_paths);

            // Also include stdout/stderr logs at the root of the tarball
            let (stdout_path, stderr_path) = self.get_output_logs();

            if stdout_path.exists() {
                archive.add_source_file(stdout_path, Some("stdout.log"));
            }

            if stderr_path.exists() {
                archive.add_source_file(stderr_path, Some("stderr.log"));
            }

            archive.pack(TarPacker::new_gz)?;

            return Ok(true);
        }

        Ok(false)
    }

    pub fn hydrate_outputs(
        &self,
        archive_file: &Path,
        workspace_root: &Path,
        output_paths: &[WorkspaceRelativePathBuf],
    ) -> miette::Result<bool> {
        if get_cache_mode().is_readable() && archive_file.exists() {
            let archive = create_archive(workspace_root, archive_file, output_paths);
            let cache_logs = self.get_output_logs();
            let stdout_log = workspace_root.join("stdout.log");
            let stderr_log = workspace_root.join("stderr.log");

            match archive.unpack(TarUnpacker::new_gz) {
                Ok(_) => {
                    if stdout_log.exists() {
                        fs::rename(&stdout_log, cache_logs.0)?;
                    }

                    if stderr_log.exists() {
                        fs::rename(&stderr_log, cache_logs.1)?;
                    }
                }
                Err(e) => {
                    warn!(
                        "Failed to hydrate outputs ({}) from cache: {}",
                        map_list(output_paths, |f| color::file(f)),
                        color::muted_light(e.to_string())
                    );

                    // Delete target outputs to ensure a clean slate
                    for output in output_paths {
                        if !glob::is_glob(output) {
                            fs::remove(output.to_path(workspace_root))?;
                        }
                    }

                    fs::remove(stdout_log)?;
                    fs::remove(stderr_log)?;
                }
            }

            return Ok(true);
        }

        Ok(false)
    }

    pub fn get_output_logs(&self) -> (PathBuf, PathBuf) {
        (
            self.get_dir().join("stdout.log"),
            self.get_dir().join("stderr.log"),
        )
    }

    /// Load the stdout.log and stderr.log files from the cache directory.
    pub fn load_output_logs(&self) -> miette::Result<(String, String)> {
        let (stdout_path, stderr_path) = self.get_output_logs();

        let stdout = if stdout_path.exists() {
            fs::read_file(stdout_path)?
        } else {
            String::new()
        };

        let stderr = if stderr_path.exists() {
            fs::read_file(stderr_path)?
        } else {
            String::new()
        };

        Ok((stdout, stderr))
    }

    /// Write stdout and stderr log files to the cache directory.
    pub fn save_output_logs(&self, stdout: String, stderr: String) -> miette::Result<()> {
        let (stdout_path, stderr_path) = self.get_output_logs();

        fs::write_file(stdout_path, stdout)?;
        fs::write_file(stderr_path, stderr)?;

        Ok(())
    }
}
