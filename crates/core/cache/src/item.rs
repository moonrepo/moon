#[macro_export]
macro_rules! cache_item {
    ($struct:ident) => {
        impl $struct {
            pub fn load(path: PathBuf) -> Result<Self, MoonError> {
                let mut item = Self::default();
                let log_target = "moon:cache:item";

                if let Some(parent) = path.parent() {
                    fs::create_dir_all(parent)?;
                }

                if get_cache_mode().is_readable() {
                    if path.exists() {
                        trace!(
                            target: log_target,
                            "Cache hit for {}, reading",
                            color::path(&path)
                        );

                        item = json::read(&path)?;
                    } else {
                        trace!(
                            target: log_target,
                            "Cache miss for {}, does not exist",
                            color::path(&path)
                        );
                    }
                }

                item.path = path;

                Ok(item)
            }

            pub fn save(&self) -> Result<(), MoonError> {
                let log_target = "moon:cache:item";

                if get_cache_mode().is_writable() {
                    trace!(
                        target: log_target,
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
