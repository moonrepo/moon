#[macro_export]
macro_rules! cache_item {
    ($struct:ident) => {
        impl $struct {
            pub async fn load(path: PathBuf, stale_ms: u128) -> Result<Self, MoonError> {
                let mut item = Self::default();

                const LOG_TARGET: &str = "moon:cache:item";

                if is_readable() {
                    if path.exists() {
                        // If stale, treat as a cache miss
                        if stale_ms > 0
                            && time::now_millis()
                                - time::to_millis(fs::metadata(&path).await?.modified().unwrap())
                                > stale_ms
                        {
                            trace!(
                                target: LOG_TARGET,
                                "Cache skip for {}, marked as stale",
                                color::path(&path)
                            );
                        } else {
                            trace!(
                                target: LOG_TARGET,
                                "Cache hit for {}, reading",
                                color::path(&path)
                            );

                            item = json::read(&path)?;
                        }
                    } else {
                        trace!(
                            target: LOG_TARGET,
                            "Cache miss for {}, does not exist",
                            color::path(&path)
                        );

                        fs::create_dir_all(path.parent().unwrap()).await?;
                    }
                }

                item.path = path;

                Ok(item)
            }

            pub async fn save(&self) -> Result<(), MoonError> {
                const LOG_TARGET: &str = "moon:cache:item";

                if is_writable() {
                    trace!(
                        target: LOG_TARGET,
                        "Writing cache {}",
                        color::path(&self.path)
                    );

                    json::write(&self.path, &self, false)?;
                }

                Ok(())
            }

            pub fn get_dir(&self) -> &Path {
                self.path.parent().unwrap()
            }
        }
    };
}
