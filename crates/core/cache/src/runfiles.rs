use moon_logger::trace;
use serde::de::DeserializeOwned;
use serde::Serialize;
use starbase_styles::color;
use starbase_utils::{fs, json};
use std::path::PathBuf;

pub struct Snapshot {
    pub path: PathBuf,
}

impl Snapshot {
    pub fn load<T: DeserializeOwned + Serialize>(
        path: PathBuf,
        data: &T,
    ) -> miette::Result<Snapshot> {
        trace!(target: "moon:cache:snapshot", "Writing snapshot {}", color::path(&path));

        fs::create_dir_all(path.parent().unwrap())?;

        // Always write a snapshot, regardless of MOON_CACHE,
        // since consumers expect this to exist at runtime
        json::write_file(&path, data, true)?;

        Ok(Snapshot { path })
    }
}
