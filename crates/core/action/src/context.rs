use clap::ValueEnum;
use rustc_hash::{FxHashMap, FxHashSet};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(ValueEnum, Clone, Debug, Deserialize, Serialize)]
pub enum ProfileType {
    Cpu,
    Heap,
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionContext {
    pub affected: bool,

    pub initial_targets: FxHashSet<String>,

    pub passthrough_args: Vec<String>,

    pub primary_targets: FxHashSet<String>,

    pub profile: Option<ProfileType>,

    pub target_hashes: FxHashMap<String, String>,

    pub touched_files: FxHashSet<PathBuf>,
}

impl ActionContext {
    pub fn should_inherit_args<T: AsRef<str>>(&self, target_id: T) -> bool {
        if self.passthrough_args.is_empty() {
            return false;
        }

        let target_id = target_id.as_ref();

        // project:task == project:task
        if self.primary_targets.contains(target_id) {
            return true;
        }

        // :task == project:task
        for initial_target in &self.initial_targets {
            if target_id.ends_with(initial_target) {
                return true;
            }
        }

        false
    }
}
