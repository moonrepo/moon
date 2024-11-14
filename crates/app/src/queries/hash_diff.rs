use super::hash::query_hash;
use moon_cache::CacheEngine;
use serde::{Deserialize, Serialize};
use tracing::debug;

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
    cache_engine: &CacheEngine,
    base_left: &str,
    base_right: &str,
) -> miette::Result<QueryHashDiffResult> {
    debug!("Diffing hashes");

    let (left_hash, left) = query_hash(cache_engine, base_left).await?;
    let (right_hash, right) = query_hash(cache_engine, base_right).await?;

    Ok(QueryHashDiffResult {
        left,
        left_hash,
        left_diffs: vec![],
        right,
        right_hash,
        right_diffs: vec![],
    })
}
