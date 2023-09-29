use clap::ValueEnum;
use moon_common::path::WorkspaceRelativePathBuf;
use moon_target::{Target, TargetLocator};
use rustc_hash::{FxHashMap, FxHashSet};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(ValueEnum, Clone, Debug, Deserialize, Serialize)]
pub enum ProfileType {
    Cpu,
    Heap,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "state", content = "hash", rename_all = "lowercase")]
pub enum TargetState {
    Completed(String),
    Failed,
    Skipped,
    Passthrough,
}

impl TargetState {
    pub fn is_complete(&self) -> bool {
        matches!(self, TargetState::Completed(_) | TargetState::Passthrough)
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionContext {
    pub affected_only: bool,

    pub initial_targets: FxHashSet<TargetLocator>,

    pub interactive: bool,

    pub passthrough_args: Vec<String>,

    pub primary_targets: FxHashSet<Target>,

    pub profile: Option<ProfileType>,

    pub target_states: FxHashMap<Target, TargetState>,

    pub touched_files: FxHashSet<WorkspaceRelativePathBuf>,

    pub workspace_root: PathBuf,
}

impl ActionContext {
    pub fn should_inherit_args<T: AsRef<Target>>(&self, target: T) -> bool {
        if self.passthrough_args.is_empty() {
            return false;
        }

        let target = target.as_ref();

        // scope:task == scope:task
        if self.primary_targets.contains(target) {
            return true;
        }

        // :task == scope:task
        for initial_target in &self.initial_targets {
            if target.is_all_task(initial_target) {
                return true;
            }
        }

        false
    }
}
