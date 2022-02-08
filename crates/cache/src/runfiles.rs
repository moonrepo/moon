use moon_error::MoonError;
use moon_utils::fs;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::path::PathBuf;

pub struct CacheRunfile {
    pub path: PathBuf,
}

impl CacheRunfile {
    pub async fn load<T: DeserializeOwned + Serialize>(
        path: PathBuf,
        data: &T,
    ) -> Result<CacheRunfile, MoonError> {
        fs::create_dir_all(path.parent().unwrap()).await?;

        if !path.exists() {
            fs::write_json(&path, data, false).await?;
        }

        Ok(CacheRunfile { path })
    }
}
