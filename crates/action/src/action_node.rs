use moon_common::Id;
use moon_target::Target;
use moon_toolchain::Runtime;
use rustc_hash::FxHashMap;
use serde::Serialize;
use std::hash::{Hash, Hasher};

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RuntimeNode {
    pub runtime: Runtime,
}

pub type InstallWorkspaceDepsNode = RuntimeNode;
pub type SetupToolchainNode = RuntimeNode;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ScopedRuntimeNode {
    pub project: Id,
    pub runtime: Runtime,
}

pub type InstallProjectDepsNode = ScopedRuntimeNode;
pub type SyncProjectNode = ScopedRuntimeNode;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RunTaskNode {
    pub args: Vec<String>,
    pub env: FxHashMap<String, String>,
    pub interactive: bool, // Interactive with stdin
    pub persistent: bool,  // Never terminates
    pub runtime: Runtime,
    pub target: Target,
    pub timeout: Option<u64>,
    pub id: Option<u32>, // For action graph states
}

impl RunTaskNode {
    pub fn new(target: Target, runtime: Runtime) -> Self {
        Self {
            args: vec![],
            env: FxHashMap::default(),
            interactive: false,
            persistent: false,
            runtime,
            target,
            timeout: None,
            id: None,
        }
    }

    fn calculate_id(&mut self) {
        let mut id = 0;

        for ch in self.target.as_str().chars() {
            if let Some(num) = ch.to_digit(10) {
                id += num;
            }
        }

        if self.persistent {
            id += 100;
        } else if self.interactive {
            id += 50;
        }

        self.id = Some(id);
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
#[serde(tag = "action", content = "params", rename_all = "kebab-case")]
pub enum ActionNode {
    #[default]
    None,

    /// Install tool dependencies in the project root.
    InstallProjectDeps(Box<InstallProjectDepsNode>),

    /// Install tool dependencies in the workspace root.
    InstallWorkspaceDeps(Box<InstallWorkspaceDepsNode>),

    /// Run a project's task.
    RunTask(Box<RunTaskNode>),

    /// Setup a tool + version for the provided toolchain.
    SetupToolchain(Box<SetupToolchainNode>),

    /// Sync a project with language specific semantics.
    SyncProject(Box<SyncProjectNode>),

    /// Sync the entire moon workspace and install system dependencies.
    SyncWorkspace,
}

impl ActionNode {
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

    pub fn setup_toolchain(node: SetupToolchainNode) -> Self {
        Self::SetupToolchain(Box::new(node))
    }

    pub fn sync_project(node: SyncProjectNode) -> Self {
        Self::SyncProject(Box::new(node))
    }

    pub fn sync_workspace() -> Self {
        Self::SyncWorkspace
    }

    pub fn get_id(&self) -> u32 {
        match self {
            Self::RunTask(inner) => inner.id.unwrap_or_default(),
            _ => 0,
        }
    }

    pub fn get_runtime(&self) -> &Runtime {
        match self {
            Self::InstallWorkspaceDeps(inner) => &inner.runtime,
            Self::InstallProjectDeps(inner) => &inner.runtime,
            Self::RunTask(inner) => &inner.runtime,
            Self::SetupToolchain(inner) => &inner.runtime,
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
            Self::InstallWorkspaceDeps(inner) => {
                format!("InstallWorkspaceDeps({})", inner.runtime.target())
            }
            Self::InstallProjectDeps(inner) => {
                format!(
                    "InstallProjectDeps({}, {})",
                    inner.runtime.target(),
                    inner.project
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
            Self::SetupToolchain(inner) => {
                if inner.runtime.platform.is_system() {
                    "SetupToolchain(system)".into()
                } else {
                    format!("SetupToolchain({})", inner.runtime.target())
                }
            }
            Self::SyncProject(inner) => {
                format!("SyncProject({}, {})", inner.runtime.id(), inner.project)
            }
            Self::SyncWorkspace => "SyncWorkspace".into(),
            Self::None => "None".into(),
        }
    }
}

impl Hash for ActionNode {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write(self.label().as_bytes());

        // For tasks with passthrough arguments and environment variables,
        // we need to ensure the hash is more unique in the graph
        if let Self::RunTask(node) = self {
            for arg in &node.args {
                state.write(arg.as_bytes());
            }

            for (key, value) in &node.env {
                state.write(key.as_bytes());
                state.write(value.as_bytes());
            }
        }
    }
}
