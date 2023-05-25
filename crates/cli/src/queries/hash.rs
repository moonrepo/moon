use moon_error::MoonError;
use moon_logger::debug;
use moon_workspace::Workspace;
use starbase::AppResult;
use starbase_styles::color;
use starbase_utils::fs;

const LOG_TARGET: &str = "moon:query:hash";

pub async fn query_hash(workspace: &Workspace, hash: &str) -> AppResult<(String, String)> {
    debug!(
        target: LOG_TARGET,
        "Querying for hash manifest with {}",
        color::hash(hash)
    );

    for file in fs::read_dir(&workspace.cache.hashes_dir)? {
        let path = file.path();
        let name = fs::file_name(&path).replace(".json", "");

        if hash == name || name.starts_with(hash) {
            debug!(
                target: LOG_TARGET,
                "Found hash manifest {} for {}",
                color::id(&name),
                color::hash(hash)
            );

            return Ok((name, fs::read_file(path)?));
        }
    }

    Err(MoonError::Generic(format!(
        "Unable to find a hash manifest for {}!",
        color::hash(hash)
    ))
    .into())
}
