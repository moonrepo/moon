use moon_common::consts::CONFIG_DIRNAME;
use rustc_hash::{FxHashMap, FxHasher};
use schematic::{Cacher, ConfigError};
use std::fs;
use std::hash::Hasher;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

pub struct ConfigCache {
    memory: FxHashMap<String, String>,
    workspace_root: PathBuf,
}

impl ConfigCache {
    pub fn new(workspace_root: &Path) -> Self {
        Self {
            memory: FxHashMap::default(),
            workspace_root: workspace_root.to_path_buf(),
        }
    }

    pub fn get_temp_path(&self, url: &str) -> PathBuf {
        let mut hasher = FxHasher::default();
        hasher.write(url.as_bytes());

        self.workspace_root
            .join(CONFIG_DIRNAME)
            .join("cache")
            .join("temp")
            .join(format!("{}.yml", hasher.finish()))
    }
}

// If reading/writing the cache fails, don't crash the entire process,
// just store in memory and move on!
impl Cacher for ConfigCache {
    fn read(&mut self, url: &str) -> Result<Option<String>, ConfigError> {
        if let Some(contents) = self.memory.get(url) {
            return Ok(Some(contents.to_owned()));
        }

        let file = self.get_temp_path(url);

        if file.exists() {
            let Ok(last_used) =
                fs::metadata(&file).and_then(|meta| meta.modified().or_else(|_| meta.created()))
            else {
                return Ok(None);
            };

            let now = SystemTime::now();
            let ttl = Duration::from_secs(86400); // 24 hours

            if last_used > (now - ttl) {
                if let Ok(contents) = fs::read_to_string(&file) {
                    self.memory.insert(url.to_owned(), contents.to_owned());

                    return Ok(Some(contents));
                }
            }
        }

        Ok(None)
    }

    fn write(&mut self, url: &str, contents: &str) -> Result<(), ConfigError> {
        if !self.memory.contains_key(url) {
            let _ = fs::write(self.get_temp_path(url), contents);

            self.memory.insert(url.to_owned(), contents.to_owned());
        }

        Ok(())
    }
}
