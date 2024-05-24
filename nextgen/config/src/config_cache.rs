use moon_common::consts::CONFIG_DIRNAME;
use schematic::{Cacher, ConfigError};
use std::fs;
use std::path::{Path, PathBuf};

pub struct ConfigCache {
    workspace_root: PathBuf,
}

impl ConfigCache {
    pub fn new(workspace_root: &Path) -> Self {
        Self {
            workspace_root: workspace_root.to_path_buf(),
        }
    }

    pub fn get_temp_path(&self, url: &str) -> PathBuf {
        self.workspace_root
            .join(CONFIG_DIRNAME)
            .join("cache")
            .join("temp")
    }
}

// If reading/writing the cache fails, don't crash the entire process,
// just log a warning and move on!
impl Cacher for ConfigCache {
    fn read(&mut self, url: &str) -> Result<Option<String>, ConfigError> {
        let file = self.get_temp_path(url);

        if file.exists() {
            match fs::read_to_string(&file) {
                Ok(data) => return Ok(Some(data)),
                Err(error) => {
                    #[cfg(feature = "tracing")]
                    {
                        use tracing::warn;

                        warn!(
                            source_url = url,
                            cache_file = ?file,
                            "Failed to read cache of external configuration file: {error}",
                        );
                    }
                }
            }
        }

        Ok(None)
    }

    fn write(&mut self, url: &str, contents: &str) -> Result<(), ConfigError> {
        let file = self.get_temp_path(url);

        if let Err(error) = fs::write(&file, contents) {
            #[cfg(feature = "tracing")]
            {
                use tracing::warn;

                warn!(
                    source_url = url,
                    cache_file = ?file,
                    "Failed to write cache of external configuration file: {error}",
                );
            }
        }

        Ok(())
    }
}
