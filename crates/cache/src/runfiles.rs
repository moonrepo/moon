use moon_error::MoonError;
use moon_logger::{color, trace};
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
        trace!(target: "moon:cache:runfile", "Writing runfile {}", color::path(&path));

        fs::create_dir_all(path.parent().unwrap()).await?;

        // Always write a runfile, regardless of MOON_CACHE,
        // since consumers expect this to exist at runtime
        fs::write_json(&path, data, true).await?;

        Ok(CacheRunfile { path })
    }
}
