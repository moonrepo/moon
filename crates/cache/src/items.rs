use crate::helpers::{is_readable, is_writable};
use moon_error::MoonError;
use moon_logger::{color, trace};
use moon_utils::{fs, time};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

const LOG_TARGET: &str = "moon:cache:item";

macro_rules! cache_item {
    ($struct:ident) => {
        impl $struct {
            pub async fn load(path: PathBuf, stale_ms: u128) -> Result<Self, MoonError> {
                let mut item = Self::default();

                if is_readable() {
                    if path.exists() {
                        // If stale, treat as a cache miss
                        if stale_ms > 0
                            && time::now_millis()
                                - time::to_millis(fs::metadata(&path).await?.modified().unwrap())
                                > stale_ms
                        {
                            trace!(
                                target: LOG_TARGET,
                                "Cache skip for {}, marked as stale",
                                color::path(&path)
                            );
                        } else {
                            trace!(
                                target: LOG_TARGET,
                                "Cache hit for {}, reading",
                                color::path(&path)
                            );

                            item = fs::read_json(&path).await?;
                        }
                    } else {
                        trace!(
                            target: LOG_TARGET,
                            "Cache miss for {}, does not exist",
                            color::path(&path)
                        );

                        fs::create_dir_all(path.parent().unwrap()).await?;
                    }
                }

                item.path = path;

                Ok(item)
            }

            pub async fn save(&self) -> Result<(), MoonError> {
                if is_writable() {
                    trace!(
                        target: LOG_TARGET,
                        "Writing cache {}",
                        color::path(&self.path)
                    );

                    fs::write_json(&self.path, &self, false).await?;
                }

                Ok(())
            }
        }
    };
}

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

#[derive(Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectsState {
    #[serde(default)]
    pub globs: Vec<String>,

    #[serde(default)]
    pub projects: HashMap<String, String>,

    #[serde(skip)]
    pub path: PathBuf,
}

cache_item!(ProjectsState);

#[derive(Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolState {
    #[serde(default)]
    pub last_version_check_time: u128,

    #[serde(skip)]
    pub path: PathBuf,
}

cache_item!(ToolState);

#[derive(Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DependenciesState {
    #[serde(default)]
    pub last_install_time: u128,

    #[serde(skip)]
    pub path: PathBuf,
}

cache_item!(DependenciesState);
