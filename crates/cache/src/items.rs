use moon_error::MoonError;
use moon_utils::fs;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::SystemTime;

pub struct CacheItem<T: DeserializeOwned + Serialize> {
    pub item: T,

    pub path: PathBuf,
}

impl<T: DeserializeOwned + Serialize> CacheItem<T> {
    pub async fn load(path: PathBuf, default: T) -> Result<CacheItem<T>, MoonError> {
        let item: T;

        if path.exists() {
            item = fs::read_json(&path).await?;
        } else {
            item = default;

            fs::create_dir_all(path.parent().unwrap()).await?;
        }

        Ok(CacheItem { item, path })
    }

    pub async fn save(&self) -> Result<(), MoonError> {
        fs::write_json(&self.path, &self.item).await?;

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

#[derive(Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetRunState {
    pub exit_code: i32,

    pub last_run_time: u128,

    pub stderr: String,

    pub stdout: String,

    pub target: String,
}

#[derive(Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceState {
    pub last_node_install_time: u128,
}
