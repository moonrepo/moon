// TODO: Improve macro so that it can be called multiple times within the same file.
// Right now the "load_config" functions cause a name collision.
#[macro_export]
macro_rules! config_cache {
    ($struct:ident, $file:expr, $reader:ident) => {
        config_cache!($struct, $file, $reader, noop_write);
    };
    ($struct:ident, $file:expr, $reader:ident, $writer:ident) => {
        fn load_config_internal(path: &Path) -> miette::Result<$struct> {
            use moon_logger::trace;
            use starbase_styles::color;

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
        #[cached(sync_writes = "default", result = true)]
        fn load_config(path: PathBuf) -> miette::Result<$struct> {
            load_config_internal(&path)
        }

        fn noop_write(_path: &Path, _file: &$struct) -> miette::Result<()> {
            Ok(()) // Do nothing
        }

        impl $struct {
            /// Read the config file from the cache. If not cached and the file exists,
            /// load it and store in the cache, otherwise return none.
            #[track_caller]
            pub fn read<P: AsRef<Path>>(path: P) -> miette::Result<Option<$struct>> {
                $struct::read_with_name(path, $file)
            }

            /// Read the config file from the cache using the provided file name.
            #[track_caller]
            pub fn read_with_name<P, N>(path: P, name: N) -> miette::Result<Option<$struct>>
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
            pub fn sync<P, F>(path: P, func: F) -> miette::Result<bool>
            where
                P: AsRef<Path>,
                F: FnOnce(&mut $struct) -> miette::Result<bool>
            {
                $struct::sync_with_name(path, $file, func)
            }

            #[track_caller]
            pub fn sync_with_name<P, N, F>(path: P, name: N, func: F) -> miette::Result<bool>
            where
                P: AsRef<Path>,
                N: AsRef<str>,
                F: FnOnce(&mut $struct) -> miette::Result<bool>
            {
                use cached::Cached;
                use moon_logger::trace;
                use starbase_styles::color;

                let mut path = path.as_ref().to_path_buf();
                let name = name.as_ref();

                if !path.ends_with(name) {
                    path = path.join(name);
                }

                // Abort early and dont acquire a lock if the config doesnt exist
                if !path.exists() {
                    return Ok(false);
                }

                let mut cache = LOAD_CONFIG.lock().unwrap();
                let mut cfg: $struct;

                if let Some(item) = cache.cache_get(&path) {
                    cfg = item.clone();
                } else {
                    cfg = load_config_internal(&path)?;
                }

                if func(&mut cfg)? {
                    trace!(
                        target: "moon:lang:config",
                        "Syncing {} with changes",
                        color::path(&path),
                    );

                    // Write to the file system
                    $writer(&path, &cfg)?;

                    // And store in the cache
                    cache.cache_set(path, cfg);

                    return Ok(true);
                }

                Ok(false)
            }

            /// Write (or overwrite) the value directly into the cache.
            #[track_caller]
            pub fn write(value: $struct) -> miette::Result<()> {
                use cached::Cached;
                use moon_logger::trace;
                use starbase_styles::color;

                let mut cache = LOAD_CONFIG.lock().unwrap();

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

#[macro_export]
macro_rules! config_cache_container {
    ($container:ident, $struct:ident, $file:expr, $reader:ident) => {
        config_cache_container!($container, $struct, $file, $reader, noop_write);
    };
    ($container:ident, $struct:ident, $file:expr, $reader:ident, $writer:path) => {
        config_cache_container!($container, $struct, $file, $reader, $writer, cache_container);
    };
    ($container:ident, $struct:ident, $file:expr, $reader:ident, $writer:path, $namespace:ident) => {
        mod $namespace {
            use super::*;

            pub fn load_config_internal(path: &Path) -> miette::Result<$struct> {
                use moon_logger::trace;
                use starbase_styles::color;

                trace!(
                    target: "moon:lang:config",
                    "Loading {}",
                    color::path(&path),
                );

                let cfg: $struct = $reader(&path)?;

                Ok(cfg)
            }

            // This merely exists to create the global cache!
            #[cached(sync_writes = "default", result = true)]
            pub fn load_config(path: PathBuf) -> miette::Result<$struct> {
                load_config_internal(&path)
            }
        }

        pub fn noop_write(_path: &Path, _file: &$struct) -> miette::Result<()> {
            Ok(()) // Do nothing
        }

        pub struct $container;

        impl $container {
            /// Read the config file from the cache. If not cached and the file exists,
            /// load it and store in the cache, otherwise return none.
            pub fn read<P: AsRef<Path>>(path: P) -> miette::Result<Option<$struct>> {
                $container::read_with_name(path, $file)
            }

            /// Read the config file from the cache using the provided file name.
            pub fn read_with_name<P, N>(path: P, name: N) -> miette::Result<Option<$struct>>
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
                    Ok(Some($namespace::load_config(path)?))
                } else {
                    Ok(None)
                }
            }

            /// If the file exists, load it from the file system, mutate it,
            /// write it back to the file system and to the cache.
            pub fn sync<P, F>(path: P, func: F) -> miette::Result<bool>
            where
                P: AsRef<Path>,
                F: FnOnce(&mut $struct) -> miette::Result<bool>
            {
                $container::sync_with_name(path, $file, func)
            }

            pub fn sync_with_name<P, N, F>(path: P, name: N, func: F) -> miette::Result<bool>
            where
                P: AsRef<Path>,
                N: AsRef<str>,
                F: FnOnce(&mut $struct) -> miette::Result<bool>
            {
                use cached::Cached;
                use moon_logger::trace;
                use starbase_styles::color;

                let mut path = path.as_ref().to_path_buf();
                let name = name.as_ref();

                if !path.ends_with(name) {
                    path = path.join(name);
                }

                // Abort early and dont acquire a lock if the config doesnt exist
                if !path.exists() {
                    return Ok(false);
                }

                let mut cache = $namespace::LOAD_CONFIG.lock().unwrap();
                let mut cfg: $struct;

                if let Some(item) = cache.cache_get(&path) {
                    cfg = item.clone();
                } else {
                    cfg = $namespace::load_config_internal(&path)?;
                }

                if func(&mut cfg)? {
                    trace!(
                        target: "moon:lang:config",
                        "Syncing {} with changes",
                        color::path(&path),
                    );

                    // Write to the file system
                    $writer(&path, &cfg)?;

                    // And store in the cache
                    cache.cache_set(path, cfg);

                    return Ok(true);
                }

                Ok(false)
            }

            /// Write (or overwrite) the value directly into the cache.
            pub fn write<P: AsRef<Path>>(path: P, value: $struct) -> miette::Result<()> {
                use cached::Cached;
                use moon_logger::trace;
                use starbase_styles::color;

                let path = path.as_ref();
                let mut cache = $namespace::LOAD_CONFIG.lock().unwrap();

                trace!(
                    target: "moon:lang:config",
                    "Writing {} to cache",
                    color::path(path),
                );

                // Write to the file system
                $writer(path, &value)?;

                // And store in the cache
                cache.cache_set(path.to_path_buf(), value);

                Ok(())
            }
        }
    };
}

#[macro_export]
macro_rules! config_cache_model {
    ($container:ident, $struct:ident, $file:expr, $reader:ident) => {
        config_cache_model!($container, $struct, $file, $reader, noop_write);
    };
    ($container:ident, $struct:ident, $file:expr, $reader:ident, $writer:path) => {
        config_cache_model!($container, $struct, $file, $reader, $writer, cache_container);
    };
    ($container:ident, $struct:ident, $file:expr, $reader:ident, $writer:path, $namespace:ident) => {
        mod $namespace {
            use super::*;

            pub fn load_config_internal(path: &Path) -> miette::Result<$struct> {
                use moon_logger::trace;
                use starbase_styles::color;

                trace!(
                    target: "moon:lang:config",
                    "Loading {}",
                    color::path(&path),
                );

                let cfg: $struct = $reader(&path)?;

                Ok(cfg)
            }

            // This merely exists to create the global cache!
            #[cached(sync_writes = "default", result = true)]
            pub fn load_config(path: PathBuf) -> miette::Result<$struct> {
                load_config_internal(&path)
            }
        }

        pub fn noop_write(_path: &Path, _file: &$struct) -> miette::Result<()> {
            Ok(()) // Do nothing
        }

        #[derive(Default)]
        pub struct $container {
            pub data: $struct,
            pub dirty: Vec<String>,
            pub path: PathBuf,
        }

        impl $container {
            /// Read the config file from the cache. If not cached and the file exists,
            /// load it and store in the cache, otherwise return none.
            pub fn read<P: AsRef<Path>>(path: P) -> miette::Result<Option<$container>> {
                $container::read_with_name(path, $file)
            }

            /// Read the config file from the cache using the provided file name.
            pub fn read_with_name<P, N>(path: P, name: N) -> miette::Result<Option<$container>>
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
                    let data = $namespace::load_config(path.clone())?;

                    Ok(Some($container {
                        data,
                        dirty: vec![],
                        path,
                    }))
                } else {
                    Ok(None)
                }
            }

            /// If the file exists, load it from the file system, mutate it,
            /// write it back to the file system and to the cache.
            pub fn sync<P, F>(path: P, func: F) -> miette::Result<bool>
            where
                P: AsRef<Path>,
                F: FnOnce(&mut $container) -> miette::Result<bool>
            {
                $container::sync_with_name(path, $file, func)
            }

            pub fn sync_with_name<P, N, F>(path: P, name: N, func: F) -> miette::Result<bool>
            where
                P: AsRef<Path>,
                N: AsRef<str>,
                F: FnOnce(&mut $container) -> miette::Result<bool>
            {
                use cached::Cached;
                use moon_logger::trace;
                use starbase_styles::color;

                let mut path = path.as_ref().to_path_buf();
                let name = name.as_ref();

                if !path.ends_with(name) {
                    path = path.join(name);
                }

                // Abort early and dont acquire a lock if the config doesnt exist
                if !path.exists() {
                    return Ok(false);
                }

                let mut cache = $namespace::LOAD_CONFIG.lock().unwrap();
                let mut data: $struct;

                if let Some(item) = cache.cache_get(&path) {
                    data = item.clone();
                } else {
                    data = $namespace::load_config_internal(&path)?;
                }

                let mut model = $container {
                    data,
                    dirty: vec![],
                    path,
                };

                if func(&mut model)? {
                    trace!(
                        target: "moon:lang:config",
                        "Syncing {} with changes",
                        color::path(&model.path),
                    );

                    // Write to the file system
                    $writer(&model.path, &model)?;

                    // And store in the cache
                    cache.cache_set(model.path, model.data);

                    return Ok(true);
                }

                Ok(false)
            }

            pub fn save(&mut self) -> miette::Result<()> {
                use cached::Cached;
                use moon_logger::trace;
                use starbase_styles::color;

                if self.dirty.is_empty() {
                    return Ok(());
                }

                trace!(
                    target: "moon:lang:config",
                    "Writing {} to cache",
                    color::path(&self.path),
                );

                // Write to the file system
                $writer(&self.path, self)?;

                // And store in the cache
                let mut cache = $namespace::LOAD_CONFIG.lock().unwrap();

                cache.cache_set(self.path.to_path_buf(), self.data.to_owned());

                self.dirty.clear();

                Ok(())
            }
        }
    };
}
