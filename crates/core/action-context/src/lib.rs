use clap::ValueEnum;
use moon_common::path::WorkspaceRelativePathBuf;
use moon_target::{Target, TargetLocator};
use rustc_hash::{FxHashMap, FxHashSet};
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, sync::Arc};

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

#[derive(Clone, Debug, Default)]
pub struct TaskNamedMutexes {
    mutexes: Arc<std::sync::Mutex<FxHashMap<String, Arc<tokio::sync::Mutex<()>>>>>,
}

impl<'de> Deserialize<'de> for TaskNamedMutexes {
    fn deserialize<D>(_: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // We don't care about unserializing this, but we need to satisfy the
        // trait requirements.
        Ok(TaskNamedMutexes::new())
    }
}

impl Serialize for TaskNamedMutexes {
    fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // We don't care about serializing this, but we need to satisfy the
        // trait requirements.
        FxHashMap::<String, ()>::default().serialize(_serializer)
    }
}

impl TaskNamedMutexes {
    fn new() -> Self {
        TaskNamedMutexes {
            mutexes: Arc::new(std::sync::Mutex::new(FxHashMap::default())),
        }
    }

    pub fn get(&self, name: &str) -> Arc<tokio::sync::Mutex<()>> {
        // TODO: Check how to remove that `unwrap`
        let mut mutexes = self.mutexes.lock().unwrap();
        if !mutexes.contains_key(name) {
            mutexes.insert(name.to_string(), Arc::new(tokio::sync::Mutex::new(())));
        }
        mutexes.get(name).unwrap().clone()
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionContext {
    pub affected_only: bool,

    pub initial_targets: FxHashSet<TargetLocator>,

    pub named_mutexes: TaskNamedMutexes,

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
        for locator in &self.initial_targets {
            if target.is_all_task(locator.as_str()) {
                return true;
            }
        }

        false
    }
}
