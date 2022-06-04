use crate::helpers::{is_readable, is_writable};
use moon_error::MoonError;
use moon_logger::{color, trace};
use moon_utils::fs;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::SystemTime;

#[derive(Debug)]
pub struct CacheItem<T: DeserializeOwned + Serialize> {
    pub item: T,

    pub path: PathBuf,
}

impl<T: DeserializeOwned + Serialize> CacheItem<T> {
    pub async fn load(path: PathBuf, default: T) -> Result<CacheItem<T>, MoonError> {
        let mut item: T = default;

        if is_readable() {
            if path.exists() {
                trace!(target: "moon:cache:item", "Cache hit for {}, reading", color::path(&path));

                item = fs::read_json(&path).await?;
            } else {
                trace!(target: "moon:cache:item", "Cache miss for {}, does not exist", color::path(&path));

                fs::create_dir_all(path.parent().unwrap()).await?;
            }
        }

        Ok(CacheItem { item, path })
    }

    pub async fn save(&self) -> Result<(), MoonError> {
        if is_writable() {
            trace!(target: "moon:cache:item", "Writing cache {}", color::path(&self.path));

            fs::write_json(&self.path, &self.item, false).await?;
        }

        Ok(())
    }

    pub fn now_millis(&self) -> u128 {
        self.to_millis(SystemTime::now())
    }

    pub fn to_millis(&self, time: SystemTime) -> u128 {
        match time.duration_since(SystemTime::UNIX_EPOCH) {
            Ok(d) => d.as_millis(),
            Err(_) => 0,
        }
    }
}

#[derive(Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunTargetState {
    pub exit_code: i32,

    pub hash: String,

    pub last_run_time: u128,

    pub stderr: String,

    pub stdout: String,

    pub target: String,
}

#[derive(Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectsState {
    #[serde(default)]
    pub projects: BTreeMap<String, String>,
}

#[derive(Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceState {
    #[serde(default)]
    pub last_node_install_time: u128,

    #[serde(default)]
    pub last_version_check_time: u128,
}
