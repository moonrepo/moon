use moon_common::Id;
use moon_platform_runtime::Runtime;
use moon_task::Target;
use serde::Serialize;
use std::hash::Hash;

#[derive(Clone, Debug, Default, Eq, Hash, PartialEq, Serialize)]
#[serde(tag = "action", content = "params")]
pub enum ActionNode {
    #[default]
    None,

    /// Install tool dependencies in the workspace root.
    InstallDeps { runtime: Runtime },

    /// Install tool dependencies in the project root.
    InstallProjectDeps { project: Id, runtime: Runtime },

    /// Run a project's task.
    RunTask {
        args: Vec<String>,
        env: Vec<(String, String)>,
        interactive: bool, // Interactive with stdin
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
            Self::InstallDeps { runtime } => runtime,
            Self::InstallProjectDeps { runtime, .. } => runtime,
            Self::RunTask { runtime, .. } => runtime,
            Self::SetupTool { runtime } => runtime,
            Self::SyncProject { runtime, .. } => runtime,
            _ => unreachable!(),
        }
    }

    pub fn is_interactive(&self) -> bool {
        match self {
            Self::RunTask { interactive, .. } => *interactive,
            _ => false,
        }
    }

    pub fn is_persistent(&self) -> bool {
        match self {
            Self::RunTask { persistent, .. } => *persistent,
            _ => false,
        }
    }

    pub fn label(&self) -> String {
        match self {
            Self::InstallDeps { runtime } => {
                format!("Install{runtime}Deps({})", runtime.requirement)
            }
            Self::InstallProjectDeps { runtime, project } => {
                format!(
                    "Install{runtime}DepsInProject({}, {project})",
                    runtime.requirement
                )
            }
            Self::RunTask {
                interactive,
                persistent,
                target,
                ..
            } => {
                format!(
                    "Run{}Task({target})",
                    if *persistent {
                        "Persistent"
                    } else if *interactive {
                        "Interactive"
                    } else {
                        ""
                    }
                )
            }
            Self::SetupTool { runtime } => {
                if runtime.platform.is_system() {
                    "SetupSystemTool".into()
                } else {
                    format!("Setup{runtime}Tool({})", runtime.requirement)
                }
            }
            Self::SyncProject { runtime, project } => {
                format!("Sync{runtime}Project({project})")
            }
            Self::SyncWorkspace => "SyncWorkspace".into(),
            Self::None => "None".into(),
        }
    }
}
