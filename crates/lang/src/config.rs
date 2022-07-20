#[macro_export]
macro_rules! config_cache {
    ($struct:ident, $file:expr, $reader:ident, $writer:ident) => {
        fn load_json(path: &Path) -> Result<$struct, MoonError> {
            use moon_logger::{color, trace};

            trace!(
                target: "moon:lang:config",
                "Loading {}",
                color::path(&path),
            );

            let mut cfg: $struct = $reader(&path)?;
            cfg.path = path.to_path_buf();

            Ok(cfg)
        }

        // This merely exists to create the global cache!
        #[cached(sync_writes = true, result = true)]
        fn load_config(path: PathBuf) -> Result<$struct, MoonError> {
            load_json(&path)
        }

        impl $struct {
            /// Read the config file from the cache. If not cached and the file exists,
            /// load it and store in the cache, otherwise return none.
            #[track_caller]
            pub fn read<P: AsRef<Path>>(path: P) -> Result<Option<$struct>, MoonError> {
                $struct::read_with_name(path, $file)
            }

            /// Read the config file from the cache using the provided file name.
            #[track_caller]
            pub fn read_with_name<P, N>(path: P, name: N) -> Result<Option<$struct>, MoonError>
            where
                P: AsRef<Path>,
                N: AsRef<str>
            {
                let mut path = path.as_ref().to_path_buf();
                let name = name.as_ref();

                if !path.ends_with(name) {
                    path = path.join(name);
                }

                if path.exists() {
                    Ok(Some(load_config(path)?))
                } else {
                    Ok(None)
                }
            }

            /// If the file exists, load it from the file system, mutate it,
            /// write it back to the file system and to the cache.
            #[track_caller]
            pub fn sync<P, F>(path: P, func: F) -> Result<bool, MoonError>
            where
                P: AsRef<Path>,
                F: FnOnce(&mut $struct) -> Result<(), MoonError>
            {
                $struct::sync_with_name(path, $file, func)
            }

            #[track_caller]
            pub fn sync_with_name<P, N, F>(path: P, name: N, func: F) -> Result<bool, MoonError>
            where
                P: AsRef<Path>,
                N: AsRef<str>,
                F: FnOnce(&mut $struct) -> Result<(), MoonError>
            {
                use cached::Cached;
                use moon_logger::{color, trace};

                let mut path = path.as_ref().to_path_buf();
                let name = name.as_ref();

                if !path.ends_with(name) {
                    path = path.join(name);
                }

                // Abort early and dont acquire a lock if the config doesnt exist
                if !path.exists() {
                    return Ok(false);
                }

                let mut cache = LOAD_CONFIG.lock().expect("Unable to acquire lock");
                let mut cfg: $struct;

                if let Some(item) = cache.cache_get(&path) {
                    cfg = item.clone();
                } else {
                    cfg = load_json(&path)?;
                }

                func(&mut cfg)?;

                trace!(
                    target: "moon:lang:config",
                    "Syncing {} with changes",
                    color::path(&path),
                );

                // Write to the file system
                $writer(&path, &cfg)?;

                // And store in the cache
                cache.cache_set(path, cfg);

                Ok(true)
            }

            /// Write (or overwrite) the value directly into the cache.
            #[track_caller]
            pub fn write(value: $struct) -> Result<(), MoonError> {
                use cached::Cached;
                use moon_logger::{color, trace};

                let mut cache = LOAD_CONFIG.lock().expect("Unable to acquire lock");

                trace!(
                    target: "moon:lang:config",
                    "Writing {} to cache",
                    color::path(&value.path),
                );

                // Write to the file system
                $writer(&value.path, &value)?;

                // And store in the cache
                cache.cache_set(value.path.clone(), value);

                Ok(())
            }
        }
    };
}
