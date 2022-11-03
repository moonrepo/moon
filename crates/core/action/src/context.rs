use clap::ValueEnum;
use rustc_hash::FxHashSet;
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
    pub initial_targets: FxHashSet<String>,

    pub passthrough_args: Vec<String>,

    pub primary_targets: FxHashSet<String>,

    pub profile: Option<ProfileType>,

    pub touched_files: FxHashSet<PathBuf>,
}

impl ActionContext {
    pub fn should_inherit_args(&self, target: &str) -> bool {
        if self.passthrough_args.is_empty() {
            return false;
        }

        // project:task == project:task
        if self.primary_targets.contains(target) {
            return true;
        }

        // :task == project:task
        for initial_target in &self.initial_targets {
            if target.ends_with(initial_target) {
                return true;
            }
        }

        false
    }
}
