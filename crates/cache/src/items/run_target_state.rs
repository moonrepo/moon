use crate::cache_item;
use crate::helpers::{is_readable, is_writable};
use moon_error::MoonError;
use moon_logger::{color, trace};
use moon_utils::{fs, time};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
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
    pub async fn load_outputs(&self) -> Result<(String, String), MoonError> {
        let stdout_path = self.path.parent().unwrap().join("stdout.log");
        let stdout = if stdout_path.exists() {
            fs::read(stdout_path).await?
        } else {
            String::new()
        };

        let stderr_path = self.path.parent().unwrap().join("stderr.log");
        let stderr = if stderr_path.exists() {
            fs::read(stderr_path).await?
        } else {
            String::new()
        };

        Ok((stdout, stderr))
    }

    pub async fn save_outputs(&self, stdout: String, stderr: String) -> Result<(), MoonError> {
        fs::write(self.path.parent().unwrap().join("stdout.log"), stdout).await?;
        fs::write(self.path.parent().unwrap().join("stderr.log"), stderr).await?;

        Ok(())
    }
}
