use moon_common::Id;
use moon_common::path::WorkspaceRelativePathBuf;
use moon_target::Target;
use moon_toolchain::{Runtime, ToolchainSpec};
use rustc_hash::{FxHashMap, FxHasher};
use serde::Serialize;
use std::hash::{Hash, Hasher};

#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallDependenciesNode {
    pub project_id: Option<Id>,
    pub root: WorkspaceRelativePathBuf,
    pub toolchain_id: Id,
}

// DEPRECATED
#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize)]
pub struct InstallWorkspaceDepsNode {
    pub runtime: Runtime,
    pub root: WorkspaceRelativePathBuf,
}

// DEPRECATED
#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallProjectDepsNode {
    pub project_id: Id,
    pub runtime: Runtime,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetupEnvironmentNode {
    pub project_id: Option<Id>,
    pub root: WorkspaceRelativePathBuf,
    pub toolchain_id: Id,
}

// DEPRECATED
#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize)]
pub struct SetupToolchainLegacyNode {
    pub runtime: Runtime,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize)]
pub struct SetupToolchainNode {
    pub spec: ToolchainSpec,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncProjectNode {
    pub project_id: Id,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RunTaskNode {
    pub args: Vec<String>,
    pub env: FxHashMap<String, String>,
    pub interactive: bool, // Interactive with stdin
    pub persistent: bool,  // Never terminates
    pub priority: u8,
    pub runtime: Runtime,
    pub target: Target,
    pub id: Option<u64>, // For action graph states
}

impl RunTaskNode {
    pub fn new(target: Target, runtime: Runtime) -> Self {
        Self {
            args: vec![],
            env: FxHashMap::default(),
            interactive: false,
            persistent: false,
            priority: 2, // normal
            runtime,
            target,
            id: None,
        }
    }

    fn calculate_id(&mut self) {
        let mut hasher = FxHasher::default();
        hasher.write(self.target.as_str().as_bytes());

        if self.persistent {
            hasher.write_u8(100);
        }

        if self.interactive {
            hasher.write_u8(50);
        }

        self.id = Some(hasher.finish());
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
#[serde(tag = "action", content = "params", rename_all = "kebab-case")]
pub enum ActionNode {
    #[default]
    None,

    /// Install toolchain dependencies in the closest root.
    InstallDependencies(Box<InstallDependenciesNode>),

    /// Install tool dependencies in the project root.
    InstallProjectDeps(Box<InstallProjectDepsNode>),

    /// Install tool dependencies in the workspace root.
    InstallWorkspaceDeps(Box<InstallWorkspaceDepsNode>),

    /// Run a project's task.
    RunTask(Box<RunTaskNode>),

    /// Setup the environment for the provided toolchain.
    SetupEnvironment(Box<SetupEnvironmentNode>),

    /// Setup a tool + version for the provided toolchain.
    SetupToolchainLegacy(Box<SetupToolchainLegacyNode>),

    /// Setup a tool + version for the provided toolchain.
    SetupToolchain(Box<SetupToolchainNode>),

    /// Sync a project with language specific semantics.
    SyncProject(Box<SyncProjectNode>),

    /// Sync the entire moon workspace and install system dependencies.
    SyncWorkspace,
}

impl ActionNode {
    pub fn install_dependencies(node: InstallDependenciesNode) -> Self {
        Self::InstallDependencies(Box::new(node))
    }

    pub fn install_project_deps(node: InstallProjectDepsNode) -> Self {
        Self::InstallProjectDeps(Box::new(node))
    }

    pub fn install_workspace_deps(node: InstallWorkspaceDepsNode) -> Self {
        Self::InstallWorkspaceDeps(Box::new(node))
    }

    pub fn run_task(mut node: RunTaskNode) -> Self {
        node.calculate_id();

        Self::RunTask(Box::new(node))
    }

    pub fn setup_environment(node: SetupEnvironmentNode) -> Self {
        Self::SetupEnvironment(Box::new(node))
    }

    pub fn setup_toolchain_legacy(node: SetupToolchainLegacyNode) -> Self {
        Self::SetupToolchainLegacy(Box::new(node))
    }

    pub fn setup_toolchain(node: SetupToolchainNode) -> Self {
        Self::SetupToolchain(Box::new(node))
    }

    pub fn sync_project(node: SyncProjectNode) -> Self {
        Self::SyncProject(Box::new(node))
    }

    pub fn sync_workspace() -> Self {
        Self::SyncWorkspace
    }

    pub fn get_id(&self) -> u64 {
        match self {
            Self::RunTask(inner) => inner.id.unwrap_or_default(),
            _ => 0,
        }
    }

    pub fn get_runtime(&self) -> &Runtime {
        match self {
            Self::InstallWorkspaceDeps(inner) => &inner.runtime,
            Self::InstallProjectDeps(inner) => &inner.runtime,
            Self::SetupToolchainLegacy(inner) => &inner.runtime,
            Self::RunTask(inner) => &inner.runtime,
            _ => unreachable!(),
        }
    }

    pub fn get_spec(&self) -> Option<&ToolchainSpec> {
        match self {
            Self::SetupToolchain(inner) => Some(&inner.spec),
            _ => None,
        }
    }

    pub fn get_priority(&self) -> u8 {
        match self {
            Self::RunTask(inner) => inner.priority,
            _ => 0,
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
            Self::InstallDependencies(inner) => {
                if inner.root.as_str().is_empty() {
                    format!("InstallDependencies({})", inner.toolchain_id)
                } else {
                    format!(
                        "InstallDependencies({}, {})",
                        inner.toolchain_id, inner.root
                    )
                }
            }
            Self::InstallWorkspaceDeps(inner) => {
                if inner.root.as_str().is_empty() {
                    format!("InstallWorkspaceDeps({})", inner.runtime.target())
                } else {
                    format!(
                        "InstallWorkspaceDeps({}, {})",
                        inner.runtime.target(),
                        inner.root
                    )
                }
            }
            Self::InstallProjectDeps(inner) => {
                format!(
                    "InstallProjectDeps({}, {})",
                    inner.runtime.target(),
                    inner.project_id
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
            Self::SetupEnvironment(inner) => {
                if inner.root.as_str().is_empty() {
                    format!("SetupEnvironment({})", inner.toolchain_id)
                } else {
                    format!("SetupEnvironment({}, {})", inner.toolchain_id, inner.root)
                }
            }
            Self::SetupToolchainLegacy(inner) => {
                if inner.runtime.is_system() {
                    "SetupToolchain(system)".into()
                } else {
                    format!("SetupToolchain({})", inner.runtime.target())
                }
            }
            Self::SetupToolchain(inner) => {
                format!("SetupToolchain({})", inner.spec.target())
            }
            Self::SyncProject(inner) => {
                format!("SyncProject({})", inner.project_id)
            }
            Self::SyncWorkspace => "SyncWorkspace".into(),
            Self::None => "None".into(),
        }
    }
}

impl Hash for ActionNode {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write(self.label().as_bytes());

        match self {
            Self::InstallDependencies(inner) => inner.hash(state),
            Self::InstallProjectDeps(inner) => inner.hash(state),
            Self::InstallWorkspaceDeps(inner) => inner.hash(state),
            Self::SetupEnvironment(inner) => inner.hash(state),
            Self::SetupToolchainLegacy(inner) => inner.hash(state),
            Self::SetupToolchain(inner) => inner.hash(state),
            Self::SyncProject(inner) => inner.hash(state),

            // For tasks with passthrough arguments and environment variables,
            // we need to ensure the hash is more unique in the graph
            Self::RunTask(inner) => {
                for arg in &inner.args {
                    state.write(arg.as_bytes());
                }

                for (key, value) in &inner.env {
                    state.write(key.as_bytes());
                    state.write(value.as_bytes());
                }
            }
            _ => {}
        };
    }
}
