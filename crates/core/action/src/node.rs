use moon_common::Id;
use moon_platform_runtime::Runtime;
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
            ActionNode::InstallDeps(platform) => {
                let version = platform.version();

                if version.is_latest() {
                    format!("Install{platform}Deps")
                } else {
                    format!("Install{platform}Deps({version})")
                }
            }
            ActionNode::InstallProjectDeps(platform, id) => {
                let version = platform.version();

                if version.is_latest() {
                    format!("Install{platform}DepsInProject({id})")
                } else {
                    format!("Install{platform}DepsInProject({version}, {id})")
                }
            }
            ActionNode::RunTarget(_, id) => format!("RunTarget({id})"),
            ActionNode::RunInteractiveTarget(_, id) => format!("RunInteractiveTarget({id})"),
            ActionNode::RunPersistentTarget(_, id) => format!("RunPersistentTarget({id})"),
            ActionNode::SetupTool(platform) => {
                let version = platform.version();

                if version.is_latest() {
                    format!("Setup{platform}Tool")
                } else {
                    format!("Setup{platform}Tool({version})")
                }
            }
            ActionNode::SyncProject(platform, id) => format!("Sync{platform}Project({id})"),
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
