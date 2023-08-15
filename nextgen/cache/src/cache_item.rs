use crate::cache_mode::get_cache_mode;
use serde::de::DeserializeOwned;
use serde::Serialize;
use starbase_utils::json;
use std::path::{Path, PathBuf};
use tracing::{debug, trace};

#[macro_export]
macro_rules! cache_item {
    ($item:item) => {
        #[derive(Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
        #[serde(default, rename_all = "camelCase")]
        $item
    };
}

pub struct CacheItem<T: Default + DeserializeOwned + Serialize> {
    pub data: T,
    pub path: PathBuf,
}

impl<T: Default + DeserializeOwned + Serialize> CacheItem<T> {
    pub fn load<P: AsRef<Path>>(path: P) -> miette::Result<CacheItem<T>> {
        let path = path.as_ref();
        let mut data = T::default();

        if get_cache_mode().is_readable() {
            if path.exists() {
                debug!(
                    cache = ?path,
                    "Cache hit, reading item",
                );

                data = json::read_file(&path)?;
            } else {
                debug!(
                    cache = ?path,
                    "Cache miss, does not exist",
                );
            }
        } else {
            trace!(
                cache = ?path,
                "Cache is not readable, skipping checks",
            );
        }

        Ok(CacheItem {
            data,
            path: path.to_path_buf(),
        })
    }

    pub fn save(&self) -> miette::Result<()> {
        if get_cache_mode().is_writable() {
            debug!(
                cache = ?self.path,
                "Writing cache item",
            );

            json::write_file(&self.path, &self.data, false)?;
        } else {
            trace!(
                cache = ?self.path,
                "Cache is not writeable, skipping save",
            );
        }

        Ok(())
    }

    pub fn get_dir(&self) -> &Path {
        self.path.parent().unwrap()
    }
}
