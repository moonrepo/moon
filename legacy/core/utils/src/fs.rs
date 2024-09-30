use crate::time::{is_stale, to_millis};
use moon_common::consts::CONFIG_DIRNAME;
use starbase_utils::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

pub mod temp {
    use super::*;

    pub fn get_dir() -> PathBuf {
        crate::get_workspace_root()
            .join(CONFIG_DIRNAME)
            .join("cache")
            .join("temp")
    }

    pub fn get_file(source: &str, ext: &str) -> PathBuf {
        get_dir().join(format!("{}.{}", crate::hash(source), ext))
    }

    pub fn read<P: AsRef<Path>>(path: P) -> miette::Result<Option<String>> {
        let file = path.as_ref();

        if !file.exists() {
            return Ok(None);
        }

        if let Ok(metadata) = file.metadata() {
            if let Ok(filetime) = metadata.created() {
                // Temp files only last for 4 hours (half a workday)
                if is_stale(to_millis(filetime), Duration::from_secs(60 * 60 * 4)) {
                    fs::remove_file(file)?;

                    return Ok(None);
                }
            }
        }

        Ok(Some(fs::read_file(file)?))
    }

    pub fn write<P: AsRef<Path>, D: AsRef<str>>(path: P, data: D) -> miette::Result<()> {
        let path = path.as_ref();

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write_file(path, data.as_ref())?;

        Ok(())
    }
}
