use moon_contract::SupportedPlatform;
use moon_project::ProjectID;
use moon_task::TargetID;
use std::hash::{Hash, Hasher};

#[derive(Clone, Eq)]
pub enum ActionNode {
    InstallDeps(SupportedPlatform),
    RunTarget(TargetID),
    SetupToolchain(SupportedPlatform),
    SyncProject(SupportedPlatform, ProjectID),
}

impl ActionNode {
    pub fn label(&self) -> String {
        match self {
            ActionNode::InstallDeps(platform) => match platform {
                SupportedPlatform::Node(version) => format!("InstallNodeDeps({})", version),
                _ => format!("Install{}Deps", platform),
            },
            ActionNode::RunTarget(id) => format!("RunTarget({})", id),
            ActionNode::SetupToolchain(platform) => match platform {
                SupportedPlatform::Node(version) => format!("SetupNodeToolchain({})", version),
                _ => format!("Setup{}Toolchain", platform),
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
