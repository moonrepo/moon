use crate::validate::{check_list, validate_child_or_root_path};
use crate::{
    language_platform::PlatformType,
    project::{PartialTaskOptionsConfig, TaskOptionsConfig},
};
use moon_target::Target;
use rustc_hash::FxHashMap;
use schematic::{config_enum, Config, ValidateError};
use shell_words::ParseError;
use strum::Display;

fn validate_inputs_outputs(list: &[String]) -> Result<(), ValidateError> {
    check_list(list, |value| validate_child_or_root_path(value))?;

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
    #[serde(untagged, expecting = "expected a string or a sequence of strings")]
    pub enum TaskCommandArgs {
        String(String),
        Sequence(Vec<String>),
    }
);

#[derive(Config)]
pub struct TaskConfig {
    pub command: Option<TaskCommandArgs>,

    pub args: Option<TaskCommandArgs>,

    pub deps: Vec<Target>,

    pub env: FxHashMap<String, String>,

    #[setting(skip)]
    pub global_inputs: Vec<String>,

    #[setting(validate = validate_inputs_outputs)]
    pub inputs: Vec<String>,

    pub local: bool,

    #[setting(validate = validate_inputs_outputs)]
    pub outputs: Vec<String>,

    #[setting(nested)]
    pub options: TaskOptionsConfig,

    pub platform: PlatformType,

    #[setting(rename = "type")]
    pub type_of: TaskType,
}

impl TaskConfig {
    pub fn get_command(&self) -> String {
        if let Some(cmd) = &self.command {
            match cmd {
                TaskCommandArgs::String(cmd_string) => {
                    let mut parts = cmd_string.split(' ');

                    if let Some(part) = parts.next() {
                        return part.to_owned();
                    }
                }
                TaskCommandArgs::Sequence(cmd_args) => {
                    if !cmd_args.is_empty() {
                        return cmd_args[0].to_owned();
                    }
                }
            };
        }

        String::new()
    }

    pub fn get_command_and_args(&self) -> Result<(Option<String>, Vec<String>), ParseError> {
        let mut command = None;
        let mut args = vec![];

        if let Some(cmd) = &self.command {
            let mut cmd_list = match cmd {
                TaskCommandArgs::String(cmd_string) => shell_words::split(cmd_string)?,
                TaskCommandArgs::Sequence(cmd_args) => cmd_args.clone(),
            };

            if !cmd_list.is_empty() {
                command = Some(cmd_list.remove(0));
                args.extend(cmd_list.clone());
            }
        }

        match &self.args {
            Some(TaskCommandArgs::String(args_string)) => {
                args.extend(shell_words::split(args_string)?)
            }
            Some(TaskCommandArgs::Sequence(args_list)) => args.extend(args_list.clone()),
            _ => {}
        }

        Ok((command, args))
    }
}
