use crate::language_platform::PlatformType;
use crate::project::{PartialTaskOptionsConfig, TaskOptionsConfig};
use crate::relative_path::RelativePath;
use moon_target::Target;
use rustc_hash::FxHashMap;
use schematic::{config_enum, Config, ValidateError};
use shell_words::ParseError;
use strum::Display;

fn validate_command<C>(
    _cmd: &TaskCommandArgs,
    task: &TaskConfig,
    _ctx: &C,
) -> Result<(), ValidateError> {
    // Only fail for empty strings and not `None`
    if let Some(cmd) = task.get_command() {
        if cmd.is_empty() {
            return Err(ValidateError::new(
                "a command is required; use \"noop\" otherwise",
            ));
        }
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

#[derive(Config)]
pub struct TaskConfig {
    #[setting(validate = validate_command)]
    pub command: TaskCommandArgs,

    pub args: TaskCommandArgs,

    pub deps: Vec<Target>,

    pub env: FxHashMap<String, String>,

    #[setting(skip)]
    pub global_inputs: Vec<RelativePath>,

    pub inputs: Vec<RelativePath>,

    pub local: bool,

    pub outputs: Vec<RelativePath>,

    #[setting(nested)]
    pub options: TaskOptionsConfig,

    pub platform: PlatformType,

    #[setting(rename = "type")]
    pub type_of: TaskType,
}

impl TaskConfig {
    pub fn get_command(&self) -> Option<String> {
        match &self.command {
            TaskCommandArgs::None => {}
            TaskCommandArgs::String(cmd_string) => {
                let mut parts = cmd_string.split(' ');

                if let Some(part) = parts.next() {
                    return Some(part.to_owned());
                }
            }
            TaskCommandArgs::Sequence(cmd_args) => {
                if !cmd_args.is_empty() {
                    return Some(cmd_args[0].to_owned());
                }
            }
        };

        None
    }

    pub fn get_command_and_args(&self) -> Result<(Option<String>, Vec<String>), ParseError> {
        let mut command = None;
        let mut args = vec![];

        let mut cmd_list = match &self.command {
            TaskCommandArgs::None => vec![],
            TaskCommandArgs::String(cmd_string) => shell_words::split(cmd_string)?,
            TaskCommandArgs::Sequence(cmd_args) => cmd_args.clone(),
        };

        if !cmd_list.is_empty() {
            command = Some(cmd_list.remove(0));
            args.extend(cmd_list.clone());
        }

        match &self.args {
            TaskCommandArgs::None => {}
            TaskCommandArgs::String(args_string) => args.extend(shell_words::split(args_string)?),
            TaskCommandArgs::Sequence(args_list) => args.extend(args_list.clone()),
        }

        Ok((command, args))
    }
}
