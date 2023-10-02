use std::hash::{Hash, Hasher};

use moon_common::Id;
use moon_platform_runtime::Runtime;

#[derive(Clone, Debug, Eq)]
pub enum ActionNode {
    /// Sync the entire moon workspace.
    /// Install system dependencies.
    SyncWorkspace,

    /// Setup a tool + version for the provided platform.
    SetupTool { runtime: Runtime },

    /// Sync a project with language specific semantics.
    SyncProject { project: Id, runtime: Runtime },
}

impl ActionNode {
    pub fn label(&self) -> String {
        String::new()
    }
}

impl PartialEq for ActionNode {
    fn eq(&self, other: &Self) -> bool {
        self.label() == other.label()
    }
}

impl Hash for ActionNode {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.label().hash(state);
    }
}
