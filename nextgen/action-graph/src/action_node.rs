use std::hash::{Hash, Hasher};

use moon_common::Id;
use moon_platform_runtime::Runtime;
use moon_task::Target;

#[derive(Clone, Debug, Eq)]
pub enum ActionNode {
    /// Install tool dependencies in the workspace root.
    InstallDeps { runtime: Runtime },

    /// Install tool dependencies in the project root.
    InstallProjectDeps { project: Id, runtime: Runtime },

    /// Run a project's task.
    RunTask {
        interactive: bool, // Interactively with stdin
        persistent: bool,  // Never terminates
        runtime: Runtime,
        target: Target,
    },

    /// Setup a tool + version for the provided platform.
    SetupTool { runtime: Runtime },

    /// Sync a project with language specific semantics.
    SyncProject { project: Id, runtime: Runtime },

    /// Sync the entire moon workspace.
    /// Install system dependencies.
    SyncWorkspace,
}

impl ActionNode {
    pub fn get_runtime(&self) -> &Runtime {
        match self {
            ActionNode::InstallDeps { runtime } => runtime,
            ActionNode::InstallProjectDeps { runtime, .. } => runtime,
            ActionNode::RunTask { runtime, .. } => runtime,
            ActionNode::SetupTool { runtime } => runtime,
            ActionNode::SyncProject { runtime, .. } => runtime,
            ActionNode::SyncWorkspace => unreachable!(),
        }
    }

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
