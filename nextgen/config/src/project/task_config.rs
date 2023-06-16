use crate::language_platform::PlatformType;
use crate::project::{PartialTaskOptionsConfig, TaskOptionsConfig};
use crate::shapes::{InputPath, OutputPath};
use moon_common::cacheable;
use moon_target::{Target, TargetScope};
use rustc_hash::FxHashMap;
use schematic::{
    derive_enum, merge, Config, ConfigEnum, ConfigError, ConfigLoader, Format, SchemaType,
    Schematic, Segment, ValidateError,
};

fn validate_command<D, C>(cmd: &TaskCommandArgs, _task: &D, _ctx: &C) -> Result<(), ValidateError> {
    let empty = match cmd {
        TaskCommandArgs::None => false,
        TaskCommandArgs::String(cmd_string) => {
            let mut parts = cmd_string.split(' ');

            if let Some(part) = parts.next() {
                part.is_empty()
            } else {
                true
            }
        }
        TaskCommandArgs::Sequence(cmd_args) => cmd_args.is_empty() || cmd_args[0].is_empty(),
    };

    // Only fail for empty strings and not `None`
    if empty {
        return Err(ValidateError::new(
            "a command is required; use \"noop\" otherwise",
        ));
    }

    Ok(())
}

pub fn validate_deps<D, C>(deps: &[Target], _data: &D, _context: &C) -> Result<(), ValidateError> {
    for (i, dep) in deps.iter().enumerate() {
        if matches!(dep.scope, TargetScope::All | TargetScope::Tag(_)) {
            return Err(ValidateError::with_segment(
                "target scope not supported as a task dependency",
                Segment::Index(i),
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

derive_enum!(
    #[derive(Default)]
    #[serde(untagged, expecting = "expected a string or a list of strings")]
    pub enum TaskCommandArgs {
        #[default]
        None,
        String(String),
        Sequence(Vec<String>),
    }
);

impl Schematic for TaskCommandArgs {
    fn generate_schema() -> SchemaType {
        let mut schema = SchemaType::union(vec![
            SchemaType::Null,
            SchemaType::string(),
            SchemaType::array(SchemaType::string()),
        ]);
        schema.set_name("TaskCommandArgs");
        schema
    }
}

cacheable!(
    #[derive(Clone, Config, Debug, Eq, PartialEq)]
    pub struct TaskConfig {
        #[setting(validate = validate_command)]
        pub command: TaskCommandArgs,

        pub args: TaskCommandArgs,

        #[setting(validate = validate_deps)]
        pub deps: Vec<Target>,

        pub env: FxHashMap<String, String>,

        #[setting(skip, merge = merge::append_vec)]
        pub global_inputs: Vec<InputPath>,

        // None = All inputs (**/*)
        // [] = No inputs
        // [...] = Specific inputs
        pub inputs: Option<Vec<InputPath>>,

        pub local: bool,

        pub outputs: Option<Vec<OutputPath>>,

        #[setting(nested)]
        pub options: TaskOptionsConfig,

        pub platform: PlatformType,

        #[serde(rename = "type")]
        pub type_of: Option<TaskType>,
    }
);

impl TaskConfig {
    pub fn parse<T: AsRef<str>>(code: T) -> Result<TaskConfig, ConfigError> {
        let result = ConfigLoader::<TaskConfig>::new()
            .code(code.as_ref(), Format::Yaml)?
            .load()?;

        Ok(result.config)
    }
}
