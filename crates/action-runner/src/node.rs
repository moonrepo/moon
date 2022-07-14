use moon_lang::SupportedLanguage;
use moon_project::ProjectID;
use moon_task::TargetID;
use std::hash::{Hash, Hasher};

#[derive(Clone, Eq)]
pub enum Node {
    InstallDeps(SupportedLanguage),
    RunTarget(TargetID),
    SetupToolchain,
    SyncProject(SupportedLanguage, ProjectID),
}

impl Node {
    pub fn label(&self) -> String {
        match self {
            Node::InstallDeps(lang) => format!("Install{}Deps", lang),
            Node::RunTarget(id) => format!("RunTarget({})", id),
            Node::SetupToolchain => "SetupToolchain".into(),
            Node::SyncProject(lang, id) => format!("Sync{}Project({})", lang, id),
        }
    }
}

impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
        self.label() == other.label()
    }
}

impl Hash for Node {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.label().hash(state);
    }
}
