use super::hash::query_hash;
use moon_logger::debug;
use moon_workspace::Workspace;
use serde::{Deserialize, Serialize};
use starbase::AppResult;

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

pub async fn query_hash_diff(
    workspace: &mut Workspace,
    options: &QueryHashDiffOptions,
) -> AppResult<QueryHashDiffResult> {
    debug!(target: LOG_TARGET, "Diffing hashes");

    let (left_hash, left) = query_hash(workspace, &options.left).await?;
    let (right_hash, right) = query_hash(workspace, &options.right).await?;

    Ok(QueryHashDiffResult {
        left,
        left_hash,
        left_diffs: vec![],
        right,
        right_hash,
        right_diffs: vec![],
    })
}
