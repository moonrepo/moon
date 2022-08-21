use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;

#[derive(ValueEnum, Clone, Debug, Deserialize, Serialize)]
pub enum ProfileType {
    Cpu,
    Heap,
}

#[derive(Default, Deserialize, Serialize)]
pub struct ActionContext {
    pub passthrough_args: Vec<String>,

    pub primary_targets: HashSet<String>,

    pub profile: Option<ProfileType>,

    pub touched_files: HashSet<PathBuf>,
}
