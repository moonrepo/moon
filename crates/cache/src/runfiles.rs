use moon_error::{map_io_to_fs_error, map_json_to_error, MoonError};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::path::PathBuf;
use tokio::fs;

pub struct CacheRunfile {
    pub path: PathBuf,
}

impl CacheRunfile {
    pub async fn load<T: DeserializeOwned + Serialize>(
        path: PathBuf,
        data: &T,
    ) -> Result<CacheRunfile, MoonError> {
        let parent = path.parent().unwrap();

        if !parent.exists() {
            fs::create_dir_all(parent)
                .await
                .map_err(|e| map_io_to_fs_error(e, parent.to_path_buf()))?;
        }

        if !path.exists() {
            let json =
                serde_json::to_string(data).map_err(|e| map_json_to_error(e, path.clone()))?;

            fs::write(&path, json)
                .await
                .map_err(|e| map_io_to_fs_error(e, path.clone()))?;
        }

        Ok(CacheRunfile { path })
    }
}
