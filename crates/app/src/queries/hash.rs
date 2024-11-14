use crate::app_error::AppError;
use moon_cache::CacheEngine;
use starbase_styles::color;
use starbase_utils::{fs, json};
use tracing::debug;

pub async fn query_hash(
    cache_engine: &CacheEngine,
    hash: &str,
) -> miette::Result<(String, String)> {
    debug!("Querying for hash manifest with {}", color::hash(hash));

    for file in fs::read_dir(&cache_engine.hash.hashes_dir)? {
        let path = file.path();
        let name = fs::file_name(&path).replace(".json", "");

        if hash == name || name.starts_with(hash) {
            debug!(
                "Found hash manifest {} for {}",
                color::id(&name),
                color::hash(hash)
            );

            // Our cache is non-pretty, but we wan't to output as pretty,
            // so we need to manually convert it here!
            let data: json::JsonValue = json::read_file(path)?;

            return Ok((name, json::format(&data, true)?));
        }
    }

    Err(AppError::MissingHashManifest(hash.to_owned()).into())
}
