use moon_common::consts::CONFIG_DIRNAME;
use moon_common::path::hash_component;
use rustc_hash::FxHashMap;
use schematic::{Cacher, HandlerError};
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

pub struct ConfigCache {
    memory: FxHashMap<String, String>,
    workspace_root: PathBuf,
}

impl ConfigCache {
    #[cfg(feature = "loader")]
    pub fn new(workspace_root: &std::path::Path) -> Self {
        Self {
            memory: FxHashMap::default(),
            workspace_root: workspace_root.to_path_buf(),
        }
    }

    pub fn get_temp_path(&self, url: &str) -> PathBuf {
        self.workspace_root
            .join(CONFIG_DIRNAME)
            .join("cache")
            .join("temp")
            .join(format!("{}.yml", hash_component(url)))
    }
}

// If reading/writing the cache fails, don't crash the entire process,
// just store in memory and move on!
impl Cacher for ConfigCache {
    fn get_file_path(&self, url: &str) -> Result<Option<PathBuf>, HandlerError> {
        let file = self.get_temp_path(url);

        Ok(if file.exists() { Some(file) } else { None })
    }

    fn read(&mut self, url: &str) -> Result<Option<String>, HandlerError> {
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

            let _ = fs::remove_file(&file);
        }

        Ok(None)
    }

    fn write(&mut self, url: &str, contents: &str) -> Result<(), HandlerError> {
        if !self.memory.contains_key(url) {
            let file = self.get_temp_path(url);

            if let Some(parent) = file.parent() {
                if !parent.exists() {
                    let _ = fs::create_dir_all(parent);
                }
            }

            let _ = fs::write(file, contents);

            self.memory.insert(url.to_owned(), contents.to_owned());
        }

        Ok(())
    }
}
