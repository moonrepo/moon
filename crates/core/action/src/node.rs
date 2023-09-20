use moon_common::Id;
use moon_platform_runtime2::Runtime;
use moon_target::Target;
use serde::Serialize;
use std::hash::{Hash, Hasher};

#[derive(Clone, Debug, Eq, Serialize)]
#[serde(tag = "action", content = "params")]
pub enum ActionNode {
    /// Install tool dependencies in the workspace root.
    InstallDeps(Runtime),

    /// Install tool dependencies in the project root.
    InstallProjectDeps(Runtime, Id),

    /// Run a target (project task).
    RunTarget(Runtime, Target),

    /// Run a target (project task) interactively with stdin.
    RunInteractiveTarget(Runtime, Target),

    /// Run a target (project task) that never terminates.
    RunPersistentTarget(Runtime, Target),

    /// Setup a tool + version for the provided platform.
    SetupTool(Runtime),

    /// Sync a project with language specific semantics.
    SyncProject(Runtime, Id),

    /// Sync the entire moon workspace.
    SyncWorkspace,
}

impl ActionNode {
    pub fn label(&self) -> String {
        match self {
            ActionNode::InstallDeps(runtime) => {
                if runtime.requirement.is_latest() {
                    format!("Install{}Deps", runtime.platform)
                } else {
                    format!("Install{}Deps({})", runtime.platform, runtime.requirement)
                }
            }
            ActionNode::InstallProjectDeps(runtime, id) => {
                if runtime.requirement.is_latest() {
                    format!("Install{}DepsInProject({id})", runtime.platform)
                } else {
                    format!(
                        "Install{}DepsInProject({}, {id})",
                        runtime.platform, runtime.requirement
                    )
                }
            }
            ActionNode::RunTarget(_, id) => format!("RunTarget({id})"),
            ActionNode::RunInteractiveTarget(_, id) => format!("RunInteractiveTarget({id})"),
            ActionNode::RunPersistentTarget(_, id) => format!("RunPersistentTarget({id})"),
            ActionNode::SetupTool(runtime) => {
                if runtime.requirement.is_latest() {
                    format!("Setup{}Tool", runtime.platform)
                } else {
                    format!("Setup{}Tool({})", runtime.platform, runtime.requirement)
                }
            }
            ActionNode::SyncProject(runtime, id) => {
                format!("Sync{}Project({id})", runtime.platform)
            }
            ActionNode::SyncWorkspace => "SyncWorkspace".into(),
        }
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
