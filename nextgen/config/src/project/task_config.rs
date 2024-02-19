use crate::language_platform::PlatformType;
use crate::project::{PartialTaskOptionsConfig, TaskOptionsConfig};
use crate::shapes::{InputPath, OutputPath};
use moon_common::{cacheable, Id};
use moon_target::{Target, TargetScope};
use rustc_hash::FxHashMap;
use schematic::{derive_enum, merge, Config, ConfigEnum, ValidateError};

fn validate_command<D, C>(
    command: &PartialTaskArgs,
    _task: &D,
    _ctx: &C,
    _finalize: bool,
) -> Result<(), ValidateError> {
    let invalid = match command {
        PartialTaskArgs::None => false,
        PartialTaskArgs::String(args) => {
            let mut parts = args.split(' ');
            let cmd = parts.next();
            cmd.is_none() || cmd.unwrap().is_empty()
        }
        PartialTaskArgs::List(args) => args.is_empty() || args[0].is_empty(),
    };

    if invalid {
        return Err(ValidateError::new(
            "a command is required; use \"noop\" otherwise",
        ));
    }

    Ok(())
}

pub(crate) fn validate_deps<D, C>(
    deps: &[PartialTaskDependency],
    _task: &D,
    _context: &C,
    _finalize: bool,
) -> Result<(), ValidateError> {
    for (i, dep) in deps.iter().enumerate() {
        let scope;

        match dep {
            PartialTaskDependency::Config(cfg) => {
                if let Some(target) = &cfg.target {
                    scope = &target.scope;
                } else {
                    return Err(ValidateError::with_segment(
                        "a target field is required",
                        schematic::PathSegment::Index(i),
                    ));
                }
            }
            PartialTaskDependency::Target(target) => {
                scope = &target.scope;
            }
        };

        if matches!(scope, TargetScope::All) {
            return Err(ValidateError::with_segment(
                "target scope not supported as a task dependency",
                schematic::PathSegment::Index(i),
            ));
        }
    }

    Ok(())
}

derive_enum!(
    /// The type of task.
    #[derive(ConfigEnum, Copy, Default)]
    pub enum TaskType {
        Build,
        Run,
        #[default]
        Test,
    }
);

cacheable!(
    /// Configures a command to execute, and its arguments.
    #[derive(Clone, Config, Debug, Eq, PartialEq)]
    #[serde(untagged, expecting = "expected a string or a list of strings")]
    pub enum TaskArgs {
        /// No value defined.
        #[setting(default, null)]
        None,
        /// A command and arguments as a string. Will be parsed into a list.
        String(String),
        /// A command and arguments as a list of individual values.
        List(Vec<String>),
    }
);

cacheable!(
    /// Expanded information about a task dependency.
    #[derive(Clone, Config, Debug, Eq, PartialEq)]
    pub struct TaskDependencyConfig {
        /// Additional arguments to pass to this dependency when it's ran.
        #[setting(nested)]
        pub args: TaskArgs,

        /// A mapping of environment variables specific to this dependency.
        pub env: FxHashMap<String, String>,

        /// The target of the depended on task.
        pub target: Target,

        /// Marks the dependency is optional when being inherited from the top-level.
        pub optional: Option<bool>,
    }
);

impl TaskDependencyConfig {
    pub fn new(target: Target) -> Self {
        Self {
            target,
            ..Default::default()
        }
    }
}

cacheable!(
    /// Configures another task that a task depends on.
    #[derive(Clone, Config, Debug, Eq, PartialEq)]
    #[serde(
        untagged,
        expecting = "expected a valid target or dependency config object"
    )]
    pub enum TaskDependency {
        /// A task referenced by target.
        Target(Target),

        /// A task referenced by target, with additional parameters to pass through.
        #[setting(nested)]
        Config(TaskDependencyConfig),
    }
);

impl TaskDependency {
    pub fn into_config(self) -> TaskDependencyConfig {
        match self {
            Self::Config(config) => config,
            Self::Target(target) => TaskDependencyConfig::new(target),
        }
    }
}

cacheable!(
    /// Configures a task to be ran within the action pipeline.
    #[derive(Clone, Config, Debug, Eq, PartialEq)]
    pub struct TaskConfig {
        /// Extends settings from a sibling task by ID.
        pub extends: Option<Id>,

        /// A human-readable description about the task.
        pub description: Option<String>,

        /// The command or command line to execute when the task is ran.
        /// Supports the command name, with or without arguments. Can be
        /// defined as a string, or a list of individual arguments.
        #[setting(nested, validate = validate_command)]
        pub command: TaskArgs,

        /// Arguments to pass to the command when it's ran. Can be
        /// defined as a string, or a list of individual arguments.
        #[setting(nested)]
        pub args: TaskArgs,

        /// Other tasks that this task depends on, and must run to completion
        /// before this task is ran. Can depend on sibling tasks, or tasks in
        /// other projects, using targets.
        #[setting(nested, validate = validate_deps)]
        pub deps: Vec<TaskDependency>,

        /// A mapping of environment variables that will be set when the
        /// task is ran.
        pub env: FxHashMap<String, String>,

        #[setting(skip, merge = merge::append_vec)]
        pub global_inputs: Vec<InputPath>,

        /// Inputs and sources that will mark the task as affected when comparing
        /// against touched files. When not provided, all files within the project
        /// are considered an input. When an empty list, no files are considered.
        /// Otherwise, an explicit list of inputs are considered.
        pub inputs: Option<Vec<InputPath>>,

        /// Marks the task as local only. Local tasks do not run in CI, do not have
        /// `options.cache` enabled, and are marked as `options.persistent`.
        pub local: Option<bool>,

        /// Outputs that will be created when the task has successfully ran.
        /// When `cache` is enabled, the outputs will be persisted for subsequent runs.
        pub outputs: Option<Vec<OutputPath>>,

        /// Options to control task inheritance and execution.
        #[setting(nested)]
        pub options: TaskOptionsConfig,

        /// The platform in which the task will be ran in. The platform determines
        /// available binaries, lookup paths, and more. When not provided, will
        /// be automatically detected.
        pub platform: PlatformType,

        /// The type of task, primarily used for categorical reasons. When not provided,
        /// will be automatically determined.
        #[serde(rename = "type")]
        pub type_of: Option<TaskType>,
    }
);

#[cfg(feature = "loader")]
impl TaskConfig {
    pub fn parse<T: AsRef<str>>(code: T) -> miette::Result<TaskConfig> {
        use moon_common::color;
        use schematic::{ConfigLoader, Format};

        let result = ConfigLoader::<TaskConfig>::new()
            .set_help(color::muted_light("https://moonrepo.dev/docs/config/tasks"))
            .code(code.as_ref(), Format::Yaml)?
            .load()?;

        Ok(result.config)
    }
}
