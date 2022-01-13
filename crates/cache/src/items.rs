use crate::errors::CacheError;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::SystemTime;
use tokio::fs;

pub struct CacheItem<T: DeserializeOwned + Serialize> {
    pub item: T,

    pub path: PathBuf,
}

impl<T: DeserializeOwned + Serialize> CacheItem<T> {
    pub async fn load(path: PathBuf, default: T) -> Result<CacheItem<T>, CacheError> {
        let item: T;

        if path.exists() {
            item = serde_json::from_str(&fs::read_to_string(&path).await?)?;
        } else {
            item = default;
        }

        Ok(CacheItem { item, path })
    }

    pub async fn save(&self) -> Result<(), CacheError> {
        fs::write(&self.path, serde_json::to_string(&self.item)?).await?;

        Ok(())
    }
}

#[derive(Default, Deserialize, Serialize)]
pub struct RunTargetItem {
    pub last_run_time: u64,

    pub target: String,
}

#[derive(Default, Deserialize, Serialize)]
pub struct WorkspaceStateItem {
    pub last_node_install: u128,
}
