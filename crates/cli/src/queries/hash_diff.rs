use crate::helpers::AnyError;
use moon_error::MoonError;
use moon_logger::{color, debug};
use moon_utils::fs;
use moon_workspace::Workspace;
use serde::{Deserialize, Serialize};
use std::path::Path;

const LOG_TARGET: &str = "moon:query:hash-diff";

#[derive(Clone, Default, Deserialize, Serialize)]
pub struct QueryHashDiffOptions {
    pub json: bool,
    pub left: String,
    pub right: String,
}

fn find_hash(dir: &Path, hash: &str) -> Result<String, MoonError> {
    for file in fs::read_dir(dir)? {
        let path = file.path();

        if fs::file_name(&path).starts_with(hash) {
            return Ok(fs::read(path)?);
        }
    }

    Err(MoonError::Generic(format!(
        "Unable to find a hash manifest for {}!",
        color::hash(hash)
    )))
}

pub async fn query_hash_diff(
    workspace: &mut Workspace,
    options: &QueryHashDiffOptions,
) -> Result<(String, String), AnyError> {
    debug!(target: LOG_TARGET, "Diffing hashes");

    let hash_left = find_hash(&workspace.cache.hashes_dir, &options.left)?;
    let hash_right = find_hash(&workspace.cache.hashes_dir, &options.right)?;

    Ok((hash_left, hash_right))
}
