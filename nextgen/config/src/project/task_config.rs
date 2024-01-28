use crate::language_platform::PlatformType;
use crate::project::{PartialTaskOptionsConfig, TaskOptionsConfig};
use crate::shapes::{InputPath, OutputPath};
use moon_common::{cacheable, Id};
use moon_target::{Target, TargetScope};
use rustc_hash::FxHashMap;
use schematic::{derive_enum, merge, Config, ConfigEnum, PathSegment, ValidateError};

fn validate_command<D, C>(
    command: &PartialTaskArgs,
    _task: &D,
    _ctx: &C,
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

pub fn validate_deps<D, C>(
    deps: &[PartialTaskDependency],
    _task: &D,
    _context: &C,
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
                        PathSegment::Index(i),
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
                PathSegment::Index(i),
            ));
        }
    }

    Ok(())
}

derive_enum!(
    #[derive(ConfigEnum, Copy, Default)]
    pub enum TaskType {
        Build,
        Run,
        #[default]
        Test,
    }
);

cacheable!(
    #[derive(Clone, Config, Debug, Eq, PartialEq)]
    #[serde(untagged, expecting = "expected a string or a list of strings")]
    pub enum TaskArgs {
        #[setting(default, null)]
        None,
        String(String),
        List(Vec<String>),
    }
);

cacheable!(
    #[derive(Clone, Config, Debug, Eq, PartialEq)]
    pub struct TaskDependencyConfig {
        #[setting(nested)]
        pub args: TaskArgs,

        pub env: FxHashMap<String, String>,

        pub target: Target,

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
    #[derive(Clone, Config, Debug, Eq, PartialEq)]
    #[serde(
        untagged,
        expecting = "expected a valid target or dependency config object"
    )]
    pub enum TaskDependency {
        Target(Target),

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
    #[derive(Clone, Config, Debug, Eq, PartialEq)]
    pub struct TaskConfig {
        pub extends: Option<Id>,

        #[setting(nested, validate = validate_command)]
        pub command: TaskArgs,

        #[setting(nested)]
        pub args: TaskArgs,

        #[setting(nested, validate = validate_deps)]
        pub deps: Vec<TaskDependency>,

        pub env: FxHashMap<String, String>,

        #[setting(skip, merge = merge::append_vec)]
        pub global_inputs: Vec<InputPath>,

        // None = All inputs (**/*)
        // [] = No inputs
        // [...] = Specific inputs
        pub inputs: Option<Vec<InputPath>>,

        pub local: Option<bool>,

        pub outputs: Option<Vec<OutputPath>>,

        #[setting(nested)]
        pub options: TaskOptionsConfig,

        pub platform: PlatformType,

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
