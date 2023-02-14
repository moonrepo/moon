use moon_platform_runtime::Runtime;
use serde::Serialize;
use std::hash::{Hash, Hasher};

#[derive(Clone, Debug, Eq, Serialize)]
#[serde(tag = "action", content = "params")]
pub enum ActionNode {
    /// Install tool dependencies in the workspace root.
    InstallDeps(Runtime),

    /// Install tool dependencies in the project root.
    InstallProjectDeps(Runtime, String),

    /// Run a target (project task).
    RunTarget(Runtime, String),

    /// Setup a tool + version for the provided platform.
    SetupTool(Runtime),

    /// Sync a project with language specific semantics.
    SyncProject(Runtime, String),
}

impl ActionNode {
    pub fn label(&self) -> String {
        match self {
            ActionNode::InstallDeps(platform) => match platform {
                Runtime::Node(version) => format!("Install{platform}Deps({version})"),
                _ => format!("Install{platform}Deps"),
            },
            ActionNode::InstallProjectDeps(platform, id) => match platform {
                Runtime::Node(version) => {
                    format!("Install{platform}DepsInProject({version}, {id})")
                }
                _ => format!("Install{platform}DepsInProject({id})"),
            },
            ActionNode::RunTarget(_, id) => format!("RunTarget({id})"),
            ActionNode::SetupTool(platform) => match platform {
                Runtime::Node(version) => format!("Setup{platform}Tool({version})"),
                _ => format!("Setup{platform}Tool"),
            },
            ActionNode::SyncProject(platform, id) => format!("Sync{platform}Project({id})"),
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
