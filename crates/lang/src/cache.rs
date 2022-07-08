#[macro_export]
macro_rules! config_cache {
    ($struct:ident) => {
        #[cached(result = true)]
        async fn load_config(path: PathBuf) -> Result<Option<$struct>, MoonError> {
            if !path.exists() {
                return Ok(None);
            }

            let mut cfg: $struct = fs::read_json(&path).await?;
            cfg.path = path;

            Ok(Some(cfg))
        }

        impl $struct {
            pub async fn read(path: PathBuf) -> Result<Option<$struct>, MoonError> {
                load_config(path).await
            }

            #[track_caller]
            pub async fn write(value: $struct) -> Result<(), MoonError> {
                use cached::Cached;

                let mut cache = LOAD_CONFIG.lock().await;
                let data = value.clone();

                cache.cache_set(data.path, Some(value));

                Ok(())
            }
        }
    };
}
