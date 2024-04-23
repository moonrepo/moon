use moon_common::Id;
use moon_platform_runtime::Runtime;
use moon_target::Target;
use serde::Serialize;
use std::hash::Hash;

#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize)]
pub struct RuntimeNode {
    pub runtime: Runtime,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize)]
pub struct ScopedRuntimeNode {
    pub project: Id,
    pub runtime: Runtime,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize)]
pub struct RunTaskNode {
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
    pub interactive: bool, // Interactive with stdin
    pub persistent: bool,  // Never terminates
    pub runtime: Runtime,
    pub target: Target,
}

#[derive(Clone, Debug, Default, Eq, Hash, PartialEq, Serialize)]
#[serde(tag = "action", content = "params")]
// 192
pub enum ActionNode {
    #[default]
    None,

    /// Install tool dependencies in the workspace root.
    InstallDeps(RuntimeNode),

    /// Install tool dependencies in the project root.
    InstallProjectDeps(ScopedRuntimeNode),

    /// Run a project's task.
    RunTask(RunTaskNode),

    /// Setup a tool + version for the provided platform.
    SetupTool(RuntimeNode),

    /// Sync a project with language specific semantics.
    SyncProject(ScopedRuntimeNode),

    /// Sync the entire moon workspace.
    /// Install system dependencies.
    SyncWorkspace,
}

impl ActionNode {
    pub fn get_runtime(&self) -> &Runtime {
        match self {
            Self::InstallDeps(inner) => &inner.runtime,
            Self::InstallProjectDeps(inner) => &inner.runtime,
            Self::RunTask(inner) => &inner.runtime,
            Self::SetupTool(inner) => &inner.runtime,
            Self::SyncProject(inner) => &inner.runtime,
            _ => unreachable!(),
        }
    }

    pub fn is_interactive(&self) -> bool {
        match self {
            Self::RunTask(inner) => inner.interactive,
            _ => false,
        }
    }

    pub fn is_persistent(&self) -> bool {
        match self {
            Self::RunTask(inner) => inner.persistent,
            _ => false,
        }
    }

    pub fn is_standard(&self) -> bool {
        match self {
            Self::RunTask(inner) => !inner.interactive && !inner.persistent,
            _ => true,
        }
    }

    pub fn label(&self) -> String {
        match self {
            Self::InstallDeps(inner) => {
                format!(
                    "Install{}Deps({})",
                    inner.runtime, inner.runtime.requirement
                )
            }
            Self::InstallProjectDeps(inner) => {
                format!(
                    "Install{}DepsInProject({}, {})",
                    inner.runtime, inner.runtime.requirement, inner.project
                )
            }
            Self::RunTask(inner) => {
                format!(
                    "Run{}Task({})",
                    if inner.persistent {
                        "Persistent"
                    } else if inner.interactive {
                        "Interactive"
                    } else {
                        ""
                    },
                    inner.target
                )
            }
            Self::SetupTool(inner) => {
                if inner.runtime.platform.is_system() {
                    "SetupSystemTool".into()
                } else {
                    format!("Setup{}Tool({})", inner.runtime, inner.runtime.requirement)
                }
            }
            Self::SyncProject(inner) => {
                format!("Sync{}Project({})", inner.runtime, inner.project)
            }
            Self::SyncWorkspace => "SyncWorkspace".into(),
            Self::None => "None".into(),
        }
    }
}
