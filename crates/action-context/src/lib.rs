use clap::ValueEnum;
use moon_affected::Affected;
use moon_common::path::WorkspaceRelativePathBuf;
use moon_target::{Target, TargetLocator};
use rustc_hash::{FxHashMap, FxHashSet};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone, Debug, Deserialize, Serialize, ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum ProfileType {
    Cpu,
    Heap,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "state", content = "hash", rename_all = "lowercase")]
pub enum TargetState {
    Passed(String), // hash
    Passthrough,    // no hash (cache off)
    Failed,
    Skipped,
}

impl TargetState {
    pub fn from_hash(hash: Option<&str>) -> Self {
        match hash {
            Some(hash) => TargetState::Passed(hash.to_string()),
            None => TargetState::Passthrough,
        }
    }

    pub fn is_complete(&self) -> bool {
        matches!(self, TargetState::Passed(_) | TargetState::Passthrough)
    }
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionContext {
    /// Projects and tasks that are affected (via `--affected`).
    pub affected: Option<Affected>,

    /// Initial target locators passed to `moon run`, `moon ci`, etc.
    pub initial_targets: FxHashSet<TargetLocator>,

    /// Active mutexes for tasks to acquire locks against.
    /// @mutable
    #[serde(skip)]
    pub named_mutexes: scc::HashMap<String, Arc<Mutex<()>>>,

    /// Additional arguments passed after `--` to passthrough.
    pub passthrough_args: Vec<String>,

    /// Targets to run after the initial locators have been resolved.
    pub primary_targets: FxHashSet<Target>,

    /// The type of profiler to run tasks with.
    pub profile: Option<ProfileType>,

    /// The current state of running tasks (via their target).
    /// @mutable
    pub target_states: scc::HashMap<Target, TargetState>,

    /// Files that have currently been touched.
    pub touched_files: FxHashSet<WorkspaceRelativePathBuf>,
}

impl ActionContext {
    pub fn get_or_create_mutex(&self, name: &str) -> Arc<Mutex<()>> {
        if let Some(value) = self.named_mutexes.read(name, |_, v| v.clone()) {
            return value;
        }

        let mutex = Arc::new(Mutex::new(()));

        let _ = self
            .named_mutexes
            .insert(name.to_owned(), Arc::clone(&mutex));

        mutex
    }

    pub fn get_target_prefix<T: AsRef<Target>>(&self, target: T) -> String {
        target.as_ref().to_prefix(
            self.primary_targets
                .iter()
                .map(|target| target.id.len())
                .max(),
        )
    }

    pub fn get_target_states(&self) -> FxHashMap<Target, TargetState> {
        let mut map = FxHashMap::default();
        self.target_states.scan(|k, v| {
            map.insert(k.to_owned(), v.to_owned());
        });
        map
    }

    pub fn is_primary_target<T: AsRef<Target>>(&self, target: T) -> bool {
        self.primary_targets.contains(target.as_ref())
    }

    pub fn set_target_state<T: AsRef<Target>>(&self, target: T, state: TargetState) {
        let _ = self.target_states.insert(target.as_ref().to_owned(), state);
    }

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
        for other_target in &self.initial_targets {
            if let TargetLocator::Qualified(other_target) = other_target {
                if other_target.is_all_task(&target.task_id) {
                    return true;
                }
            }
        }

        false
    }
}
