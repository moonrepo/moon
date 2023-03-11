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

#[derive(Clone, Default, Deserialize, Serialize)]
pub struct QueryHashDiffResult {
    pub left: String,
    pub left_hash: String,
    pub left_diffs: Vec<String>,
    pub right: String,
    pub right_hash: String,
    pub right_diffs: Vec<String>,
}

fn find_hash(dir: &Path, hash: &str) -> Result<(String, String), MoonError> {
    for file in fs::read_dir(dir)? {
        let path = file.path();
        let name = fs::file_name(&path).replace(".json", "");

        if hash == name || name.starts_with(hash) {
            debug!(
                target: LOG_TARGET,
                "Found manifest {} for hash {}",
                color::hash(&name),
                color::id(&hash)
            );

            return Ok((name, fs::read(path)?));
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
) -> Result<QueryHashDiffResult, AnyError> {
    debug!(target: LOG_TARGET, "Diffing hashes");

    let (left_hash, left) = find_hash(&workspace.cache.hashes_dir, &options.left)?;
    let (right_hash, right) = find_hash(&workspace.cache.hashes_dir, &options.right)?;

    Ok(QueryHashDiffResult {
        left,
        left_hash,
        left_diffs: vec![],
        right,
        right_hash,
        right_diffs: vec![],
    })
}
