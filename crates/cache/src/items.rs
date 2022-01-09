use serde::{Deserialize, Serialize};

pub enum CacheItem {
    RunTarget(RunTargetItem),
}

#[derive(Deserialize, Serialize)]
pub struct RunTargetItem {
    pub last_run_time: u64,

    pub target: String,
}
