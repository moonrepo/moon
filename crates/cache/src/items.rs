use crate::helpers::{is_readable, is_writable, to_millis};
use moon_error::MoonError;
use moon_logger::{color, trace};
use moon_utils::fs;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::SystemTime;

const LOG_TARGET: &str = "moon:cache:item";

pub struct CacheItem<T: DeserializeOwned + Serialize> {
    pub item: T,

    pub path: PathBuf,
}

impl<T: DeserializeOwned + Serialize> CacheItem<T> {
    pub async fn load(
        path: PathBuf,
        default: T,
        stale_ms: u128,
    ) -> Result<CacheItem<T>, MoonError> {
        let mut item: T = default;

        if is_readable() {
            if path.exists() {
                // If stale, treat as a cache miss
                if stale_ms > 0
                    && to_millis(SystemTime::now())
                        - to_millis(fs::metadata(&path).await?.modified().unwrap())
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

        Ok(CacheItem { item, path })
    }

    pub async fn save(&self) -> Result<(), MoonError> {
        if is_writable() {
            trace!(
                target: LOG_TARGET,
                "Writing cache {}",
                color::path(&self.path)
            );

            fs::write_json(&self.path, &self.item, false).await?;
        }

        Ok(())
    }

    pub fn now_millis(&self) -> u128 {
        to_millis(SystemTime::now())
    }

    pub fn to_millis(&self, time: SystemTime) -> u128 {
        to_millis(time)
    }
}

#[derive(Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunTargetState {
    pub exit_code: i32,

    pub hash: String,

    pub last_run_time: u128,

    pub stderr: String,

    pub stdout: String,

    pub target: String,
}

#[derive(Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectsState {
    #[serde(default)]
    pub globs: Vec<String>,

    #[serde(default)]
    pub projects: HashMap<String, String>,
}

#[derive(Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolState {
    #[serde(default)]
    pub last_deps_install_time: u128,

    #[serde(default)]
    pub last_version_check_time: u128,
}
