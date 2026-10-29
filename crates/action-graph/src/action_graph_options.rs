use moon_affected::Depth;
use moon_common::is_ci;
use moon_config::PipelineActionSwitch;
use moon_task::Target;
use petgraph::graph::NodeIndex;

#[derive(Clone, Debug)]
pub struct RunRequirements {
    pub ci: bool,                 // Are we in a CI environment
    pub ci_check: bool,           // Check the `runInCI` option
    pub no_dependencies: bool,    // Don't insert dependency tasks that weren't selected
    pub dependents: Depth,        // Add dependent tasks of the selected tasks
    pub interactive: bool,        // Entire pipeline is interactive
    pub job: Option<usize>,       // Current job index
    pub job_total: Option<usize>, // Total amount of jobs
    pub skip_affected: bool,      // Skip all affected checks
    pub skip_persistent: bool,    // Skip persistent tasks that aren't a dependency
}

impl Default for RunRequirements {
    fn default() -> Self {
        Self {
            ci: is_ci(),
            ci_check: false,
            no_dependencies: false,
            dependents: Depth::NONE,
            interactive: false,
            job: None,
            job_total: None,
            skip_affected: false,
            skip_persistent: false,
        }
    }
}

#[derive(Debug, Default)]
pub struct RunPartition {
    /// The node of each primary target, in the order they were selected:
    /// requested targets first, then their dependents.
    pub targets: Vec<(NodeIndex, Target)>,
    pub size: Option<usize>,
}

/// Why a task is being inserted into the graph.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum TaskOrigin {
    /// Selected by the target locators.
    #[default]
    Requested,
    /// Added as a dependent of a selected task.
    Dependent,
    /// Inserted because another task in the graph depends on it.
    Dependency,
}

pub struct ActionGraphBuilderOptions {
    pub install_dependencies: PipelineActionSwitch,
    pub setup_environment: PipelineActionSwitch,
    pub setup_toolchains: PipelineActionSwitch,
    pub sync_projects: PipelineActionSwitch,
    pub sync_project_dependencies: bool,
    pub sync_workspace: bool,
}

impl Default for ActionGraphBuilderOptions {
    fn default() -> Self {
        Self::new(true)
    }
}

impl ActionGraphBuilderOptions {
    pub fn new(state: bool) -> Self {
        Self {
            install_dependencies: state.into(),
            setup_environment: state.into(),
            setup_toolchains: state.into(),
            sync_projects: state.into(),
            sync_project_dependencies: state,
            sync_workspace: state,
        }
    }
}
