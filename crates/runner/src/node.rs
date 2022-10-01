use moon_contract::SupportedPlatform;
use moon_project::ProjectID;
use moon_task::TargetID;
use std::hash::{Hash, Hasher};

#[derive(Clone, Debug, Eq)]
pub enum ActionNode {
    /// Install tool dependencies in the workspace root.
    InstallDeps(SupportedPlatform),

    /// Install tool dependencies in the project's root.
    InstallProjectDeps(SupportedPlatform, ProjectID),

    /// Run a target (project task).
    RunTarget(TargetID),

    /// Setup a tool + version for the provided platform.
    SetupTool(SupportedPlatform),

    /// Sync a project with language specific semantics.
    SyncProject(SupportedPlatform, ProjectID),
}

impl ActionNode {
    pub fn label(&self) -> String {
        match self {
            ActionNode::InstallDeps(platform) => match platform {
                SupportedPlatform::Node(version) => format!("Install{}Deps({})", platform, version),
                _ => format!("Install{}Deps", platform),
            },
            ActionNode::InstallProjectDeps(platform, id) => match platform {
                SupportedPlatform::Node(version) => {
                    format!("Install{}DepsInProject({}, {})", platform, version, id)
                }
                _ => format!("Install{}DepsInProject({})", platform, id),
            },
            ActionNode::RunTarget(id) => format!("RunTarget({})", id),
            ActionNode::SetupTool(platform) => match platform {
                SupportedPlatform::Node(version) => format!("Setup{}Tool({})", platform, version),
                _ => format!("Setup{}Tool", platform),
            },
            ActionNode::SyncProject(platform, id) => format!("Sync{}Project({})", platform, id),
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
