use crate::validators::validate_child_or_root_path;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use validator::{Validate, ValidationError};

fn validate_inputs(list: &[String]) -> Result<(), ValidationError> {
    for (index, item) in list.iter().enumerate() {
        validate_child_or_root_path(&format!("inputs[{}]", index), item)?;
    }

    Ok(())
}

fn validate_outputs(list: &[String]) -> Result<(), ValidationError> {
    for (index, item) in list.iter().enumerate() {
        validate_child_or_root_path(&format!("outputs[{}]", index), item)?;
    }

    Ok(())
}

pub type Tasks = HashMap<String, TaskConfig>;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskType {
    Npm,
    Shell,
}

impl Default for TaskType {
    fn default() -> Self {
        TaskType::Npm
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskMergeStrategy {
    Append,
    Prepend,
    Replace,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, Validate)]
pub struct TaskOptionsConfig {
    #[serde(rename = "mergeStrategy")]
    pub merge_strategy: Option<TaskMergeStrategy>,

    #[serde(rename = "retryCount")]
    pub retry_count: Option<u8>,
}

impl Default for TaskOptionsConfig {
    fn default() -> Self {
        TaskOptionsConfig {
            merge_strategy: Some(TaskMergeStrategy::Append),
            retry_count: Some(0),
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize, Validate)]
pub struct TaskConfig {
    pub args: Option<Vec<String>>,

    pub command: String,

    #[validate(custom = "validate_inputs")]
    pub inputs: Option<Vec<String>>,

    pub options: Option<TaskOptionsConfig>,

    #[validate(custom = "validate_outputs")]
    pub outputs: Option<Vec<String>>,

    #[serde(rename = "type")]
    pub type_of: Option<TaskType>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::errors::map_figment_error_to_validation_errors;
    use crate::errors::tests::handled_jailed_error;
    use figment::{
        providers::{Format, Yaml},
        Figment,
    };
    use std::path::PathBuf;

    const CONFIG_FILENAME: &str = "tasks.yml";

    // Not a config file, but we want to test in isolation
    fn load_jailed_config() -> Result<TaskConfig, figment::Error> {
        let config: TaskConfig = match Figment::new()
            .merge(Yaml::file(&PathBuf::from(CONFIG_FILENAME)))
            .extract()
        {
            Ok(cfg) => cfg,
            Err(error) => {
                return Err(handled_jailed_error(
                    &map_figment_error_to_validation_errors(&error),
                ))
            }
        };

        Ok(config)
    }

    mod command {
        #[test]
        #[should_panic(expected = "Missing field `command`.")]
        fn missing_command() {
            figment::Jail::expect_with(|jail| {
                jail.create_file(super::CONFIG_FILENAME, "fake: value")?;

                super::load_jailed_config()?;

                Ok(())
            });
        }

        #[test]
        #[should_panic(
            expected = "Invalid field `command`. Expected a string type, received unsigned int `123`."
        )]
        fn invalid_type() {
            figment::Jail::expect_with(|jail| {
                jail.create_file(super::CONFIG_FILENAME, "command: 123")?;

                super::load_jailed_config()?;

                Ok(())
            });
        }
    }

    mod args {
        #[test]
        #[should_panic(
            expected = "Invalid field `args`. Expected a sequence type, received string \"abc\"."
        )]
        fn invalid_type() {
            figment::Jail::expect_with(|jail| {
                jail.create_file(
                    super::CONFIG_FILENAME,
                    r#"
command: foo
args: abc
"#,
                )?;

                super::load_jailed_config()?;

                Ok(())
            });
        }

        #[test]
        #[should_panic(
            expected = "Invalid field `args.0`. Expected a string type, received unsigned int `123`."
        )]
        fn invalid_value_type() {
            figment::Jail::expect_with(|jail| {
                jail.create_file(
                    super::CONFIG_FILENAME,
                    r#"
command: foo
args:
    - 123
"#,
                )?;

                super::load_jailed_config()?;

                Ok(())
            });
        }
    }

    mod inputs {
        #[test]
        #[should_panic(
            expected = "Invalid field `inputs`. Expected a sequence type, received string \"abc\"."
        )]
        fn invalid_type() {
            figment::Jail::expect_with(|jail| {
                jail.create_file(
                    super::CONFIG_FILENAME,
                    r#"
command: foo
inputs: abc
"#,
                )?;

                super::load_jailed_config()?;

                Ok(())
            });
        }

        #[test]
        #[should_panic(
            expected = "Invalid field `inputs.0`. Expected a string type, received unsigned int `123`."
        )]
        fn invalid_value_type() {
            figment::Jail::expect_with(|jail| {
                jail.create_file(
                    super::CONFIG_FILENAME,
                    r#"
command: foo
inputs:
    - 123
"#,
                )?;

                super::load_jailed_config()?;

                Ok(())
            });
        }
    }

    mod outputs {
        #[test]
        #[should_panic(
            expected = "Invalid field `outputs`. Expected a sequence type, received string \"abc\"."
        )]
        fn invalid_type() {
            figment::Jail::expect_with(|jail| {
                jail.create_file(
                    super::CONFIG_FILENAME,
                    r#"
command: foo
outputs: abc
"#,
                )?;

                super::load_jailed_config()?;

                Ok(())
            });
        }

        #[test]
        #[should_panic(
            expected = "Invalid field `outputs.0`. Expected a string type, received unsigned int `123`."
        )]
        fn invalid_value_type() {
            figment::Jail::expect_with(|jail| {
                jail.create_file(
                    super::CONFIG_FILENAME,
                    r#"
command: foo
outputs:
    - 123
"#,
                )?;

                super::load_jailed_config()?;

                Ok(())
            });
        }
    }

    mod type_of {
        #[test]
        #[should_panic(expected = "Invalid field `type`. Unknown option `unknown`.")]
        fn invalid_type() {
            figment::Jail::expect_with(|jail| {
                jail.create_file(
                    super::CONFIG_FILENAME,
                    r#"
command: foo
type: unknown
"#,
                )?;

                super::load_jailed_config()?;

                Ok(())
            });
        }
    }

    mod options {
        #[test]
        #[should_panic(
            expected = "Invalid field `options`. Expected struct TaskOptionsConfig type, received unsigned int `123`."
        )]
        fn invalid_type() {
            figment::Jail::expect_with(|jail| {
                jail.create_file(
                    super::CONFIG_FILENAME,
                    r#"
command: foo
options: 123
"#,
                )?;

                super::load_jailed_config()?;

                Ok(())
            });
        }

        #[test]
        #[should_panic(
            expected = "Invalid field `options.mergeStrategy`. Unknown option `bubble`."
        )]
        fn invalid_merge_strategy_type() {
            figment::Jail::expect_with(|jail| {
                jail.create_file(
                    super::CONFIG_FILENAME,
                    r#"
command: foo
options:
    mergeStrategy: bubble
"#,
                )?;

                super::load_jailed_config()?;

                Ok(())
            });
        }

        #[test]
        #[should_panic(
            expected = "Invalid field `options.retryCount`. Expected u8 type, received string \"abc\"."
        )]
        fn invalid_retry_count_type() {
            figment::Jail::expect_with(|jail| {
                jail.create_file(
                    super::CONFIG_FILENAME,
                    r#"
command: foo
options:
    retryCount: abc
"#,
                )?;

                super::load_jailed_config()?;

                Ok(())
            });
        }
    }
}
