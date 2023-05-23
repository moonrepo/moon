use crate::language_platform::PlatformType;
use crate::portable_path::PortablePath;
use crate::project::{PartialTaskOptionsConfig, TaskOptionsConfig};
use moon_target::Target;
use rustc_hash::FxHashMap;
use schematic::{config_enum, Config, ValidateError};
use strum::Display;

fn validate_command<C>(
    cmd: &TaskCommandArgs,
    _task: &TaskConfig,
    _ctx: &C,
) -> Result<(), ValidateError> {
    // Only fail for empty strings and not `None`
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

    if empty {
        return Err(ValidateError::new(
            "a command is required; use \"noop\" otherwise",
        ));
    }

    Ok(())
}

config_enum!(
    #[derive(Default, Display)]
    pub enum TaskType {
        #[strum(serialize = "build")]
        Build,

        #[strum(serialize = "run")]
        Run,

        #[default]
        #[strum(serialize = "test")]
        Test,
    }
);

config_enum!(
    #[derive(Default)]
    #[serde(untagged, expecting = "expected a string or a sequence of strings")]
    pub enum TaskCommandArgs {
        #[default]
        None,
        String(String),
        Sequence(Vec<String>),
    }
);

#[derive(Debug, Clone, Config)]
pub struct TaskConfig {
    #[setting(validate = validate_command)]
    pub command: TaskCommandArgs,

    pub args: TaskCommandArgs,

    pub deps: Vec<Target>,

    pub env: FxHashMap<String, String>,

    // TODO
    #[setting(skip)]
    pub global_inputs: Vec<PortablePath>,

    pub inputs: Vec<PortablePath>,

    pub local: bool,

    pub outputs: Vec<PortablePath>,

    #[setting(nested)]
    pub options: TaskOptionsConfig,

    pub platform: PlatformType,

    #[setting(rename = "type")]
    pub type_of: TaskType,
}
