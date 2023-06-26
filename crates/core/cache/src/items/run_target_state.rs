use crate::cache_item;
use crate::helpers::get_cache_mode;

use moon_archive::{untar_with_diff, TarArchiver, TreeDiffer};
use moon_common::path::WorkspaceRelativePathBuf;
use moon_logger::{map_list, trace, warn};
use serde::{Deserialize, Serialize};
use starbase_styles::color;
use starbase_utils::{fs, glob, json};
use std::path::{Path, PathBuf};
use std::{thread, time};

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

impl RunTargetState {
    pub fn archive_outputs(
        &self,
        archive_file: &Path,
        workspace_root: &Path,
        output_paths: &[WorkspaceRelativePathBuf],
    ) -> miette::Result<bool> {
        if get_cache_mode().is_writable() && !archive_file.exists() {
            let mut tar = TarArchiver::new(workspace_root, archive_file);

            // Outputs are relative from project root (the input)
            if !output_paths.is_empty() {
                for output in output_paths {
                    if glob::is_glob(output) {
                        tar.add_source_glob(output.as_str(), None);
                    } else {
                        tar.add_source(output.to_path(workspace_root), Some(output.as_str()));
                    }
                }
            }

            // Also include stdout/stderr logs at the root of the tarball
            let (stdout_path, stderr_path) = self.get_output_logs();

            if stdout_path.exists() {
                tar.add_source(stdout_path, Some("stdout.log"));
            }

            if stderr_path.exists() {
                tar.add_source(stderr_path, Some("stderr.log"));
            }

            tar.pack()?;

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
            let tarball_file = archive_file.to_path_buf();
            let workspace_root = workspace_root.to_path_buf();
            let cache_logs = self.get_output_logs();
            let outputs = output_paths
                .iter()
                .map(|o| o.as_str().to_string())
                .collect::<Vec<_>>();

            // Run in a separate thread so that if the current thread aborts,
            // we don't stop hydration partially though, resulting in a
            // corrupted cache.
            tokio::spawn(async move {
                let mut differ = TreeDiffer::load(&workspace_root, &outputs)?;
                let stdout_log = workspace_root.join("stdout.log");
                let stderr_log = workspace_root.join("stderr.log");

                match untar_with_diff(&mut differ, tarball_file, &workspace_root, None) {
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
                            map_list(&outputs, |f| color::file(f)),
                            color::muted_light(e.to_string())
                        );

                        // Delete target outputs to ensure a clean slate
                        for output in outputs {
                            fs::remove(workspace_root.join(output))?;
                        }

                        fs::remove(stdout_log)?;
                        fs::remove(stderr_log)?;
                    }
                }

                Ok::<(), miette::Report>(())
            });

            // Attempt to emulate how long it would take to unpack the archive
            // based on its filesize. We do this so that subsequent tasks that
            // depend on this output aren't interacting with it before it's
            // entirely unpacked.
            if let Ok(meta) = fs::metadata(archive_file) {
                let size = meta.len();
                let millis = (size / 1000000) * 10;

                thread::sleep(time::Duration::from_millis(millis));
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
