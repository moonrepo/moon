#[macro_export]
macro_rules! config_cache {
    ($struct:ident, $writer:ident) => {
        async fn load_json(path: &Path) -> Result<$struct, MoonError> {
            use moon_logger::{color, trace};

            trace!(
                target: "moon:lang:config",
                "Loading {}",
                color::path(&path),
            );

            let mut cfg: $struct = fs::read_json(&path).await?;
            cfg.path = path.to_path_buf();

            Ok(cfg)
        }

        // This merely exists to create the global cache!
        #[cached(sync_writes = true, result = true)]
        async fn load_config(path: PathBuf) -> Result<$struct, MoonError> {
            load_json(&path).await
        }

        impl $struct {
            /// Read the config file from the cache. If not cached, and the file exists
            /// load it and store in the cache, otherwise return none.
            #[track_caller]
            pub async fn read(path: PathBuf) -> Result<Option<$struct>, MoonError> {
                if path.exists() {
                    Ok(Some(load_config(path).await?))
                } else {
                    Ok(None)
                }
            }

            #[track_caller]
            pub async fn sync<F>(path: PathBuf, func: F) -> Result<bool, MoonError>
            where
                F: FnOnce(&mut $struct)
            {
                use cached::Cached;
                use moon_logger::{color, trace};

                // Abort early and dont acquire a lock if the config doesnt exist
                if !path.exists() {
                    return Ok(false);
                }

                let mut cache = LOAD_CONFIG.lock().await;
                let mut cfg: $struct;

                if let Some(item) = cache.cache_get(&path) {
                    cfg = item.clone();
                } else {
                    cfg = load_json(&path).await?;
                }

                func(&mut cfg);

                trace!(
                    target: "moon:lang:config",
                    "Syncing {} with changes",
                    color::path(&path),
                );

                // Write to the file system
                $writer(&path, &cfg).await?;

                // And store in the cache
                cache.cache_set(path, cfg);

                Ok(true)
            }

            /// Write (or overwrite) the value directly into the cache.
            #[track_caller]
            pub async fn write(value: $struct) -> Result<(), MoonError> {
                use cached::Cached;
                use moon_logger::{color, trace};

                let mut cache = LOAD_CONFIG.lock().await;

                trace!(
                    target: "moon:lang:config",
                    "Writing {} to cache",
                    color::path(&value.path),
                );

                // Write to the file system
                $writer(&value.path, &value).await?;

                // And store in the cache
                cache.cache_set(value.path.clone(), value);

                Ok(())
            }
        }
    };
}
