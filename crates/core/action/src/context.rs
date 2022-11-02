use clap::ValueEnum;
use rustc_hash::FxHashSet;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(ValueEnum, Clone, Debug, Deserialize, Serialize)]
pub enum ProfileType {
    Cpu,
    Heap,
}

#[derive(Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionContext {
    pub passthrough_args: Vec<String>,

    pub primary_targets: FxHashSet<String>,

    pub profile: Option<ProfileType>,

    pub touched_files: FxHashSet<PathBuf>,
}
