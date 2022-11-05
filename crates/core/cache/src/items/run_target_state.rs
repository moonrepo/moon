use crate::cache_item;
use crate::helpers::{is_readable, is_writable};
use moon_archive::{untar, TarArchiver};
use moon_error::MoonError;
use moon_logger::{color, trace};
use moon_utils::{fs, json, time};
use serde::{Deserialize, Serialize};
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

impl RunTargetState {
    pub async fn archive_outputs(
        &self,
        archive_file: &Path,
        input_root: &Path,
        outputs: &[String],
    ) -> Result<bool, MoonError> {
        if is_writable() && !outputs.is_empty() && !archive_file.exists() {
            let mut tar = TarArchiver::new(input_root, archive_file);

            // Outputs are relative from project root (the input)
            for output in outputs {
                tar.add_source(input_root.join(output), Some(output));
            }

            // Also include stdout/stderr logs at the root of the tarball
            let (stdout_path, stderr_path) = self.get_output_logs();

            if stdout_path.exists() {
                tar.add_source(stdout_path, Some("stdout.log"));
            }

            if stderr_path.exists() {
                tar.add_source(stderr_path, Some("stderr.log"));
            }

            tar.pack().map_err(|e| MoonError::Generic(e.to_string()))?;

            return Ok(true);
        }

        Ok(false)
    }

    pub async fn hydrate_outputs(
        &self,
        archive_file: &Path,
        project_root: &Path,
    ) -> Result<bool, MoonError> {
        if is_readable() && archive_file.exists() {
            untar(archive_file, project_root, None)
                .map_err(|e| MoonError::Generic(e.to_string()))?;

            let cache_logs = self.get_output_logs();
            let stdout_log = project_root.join("stdout.log");
            let stderr_log = project_root.join("stderr.log");

            if stdout_log.exists() {
                fs::rename(&stdout_log, cache_logs.0).await?;
            }

            if stderr_log.exists() {
                fs::rename(&stderr_log, cache_logs.1).await?;
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
    pub async fn load_output_logs(&self) -> Result<(String, String), MoonError> {
        let (stdout_path, stderr_path) = self.get_output_logs();

        let stdout = if stdout_path.exists() {
            fs::read(stdout_path).await?
        } else {
            String::new()
        };

        let stderr = if stderr_path.exists() {
            fs::read(stderr_path).await?
        } else {
            String::new()
        };

        Ok((stdout, stderr))
    }

    /// Write stdout and stderr log files to the cache directory.
    pub async fn save_output_logs(&self, stdout: String, stderr: String) -> Result<(), MoonError> {
        let (stdout_path, stderr_path) = self.get_output_logs();

        fs::write(stdout_path, stdout).await?;
        fs::write(stderr_path, stderr).await?;

        Ok(())
    }
}
