use moon_common::Id;
use moon_platform_runtime::Runtime;
use moon_target::Target;
use rustc_hash::FxHashMap;
use serde::Serialize;
use std::hash::{Hash, Hasher};

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RuntimeNode {
    pub runtime: Runtime,
}

pub type InstallDepsNode = RuntimeNode;
pub type SetupToolNode = RuntimeNode;

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

    /// Install tool dependencies in the workspace root.
    InstallDeps(Box<InstallDepsNode>),

    /// Install tool dependencies in the project root.
    InstallProjectDeps(Box<InstallProjectDepsNode>),

    /// Run a project's task.
    RunTask(Box<RunTaskNode>),

    /// Setup a tool + version for the provided platform.
    SetupTool(Box<SetupToolNode>),

    /// Sync a project with language specific semantics.
    SyncProject(Box<SyncProjectNode>),

    /// Sync the entire moon workspace and install system dependencies.
    SyncWorkspace,
}

impl ActionNode {
    pub fn install_deps(node: InstallDepsNode) -> Self {
        Self::InstallDeps(Box::new(node))
    }

    pub fn install_project_deps(node: InstallProjectDepsNode) -> Self {
        Self::InstallProjectDeps(Box::new(node))
    }

    pub fn run_task(mut node: RunTaskNode) -> Self {
        node.calculate_id();

        Self::RunTask(Box::new(node))
    }

    pub fn setup_tool(node: SetupToolNode) -> Self {
        Self::SetupTool(Box::new(node))
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
