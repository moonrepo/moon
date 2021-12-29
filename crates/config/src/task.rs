use crate::validators::{validate_child_or_root_path, validate_target};
use serde::{Deserialize, Serialize};
use validator::{Validate, ValidationError};

fn validate_deps(list: &[String]) -> Result<(), ValidationError> {
    for (index, item) in list.iter().enumerate() {
        validate_target(&format!("deps[{}]", index), item)?;
    }

    Ok(())
}

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

// project_id:task_name
pub type Target = String;

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

impl Default for TaskMergeStrategy {
    fn default() -> Self {
        TaskMergeStrategy::Append
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct TaskOptionsConfig {
    pub merge_args: Option<TaskMergeStrategy>,

    pub merge_deps: Option<TaskMergeStrategy>,

    pub merge_inputs: Option<TaskMergeStrategy>,

    pub merge_outputs: Option<TaskMergeStrategy>,

    pub retry_count: Option<u8>,

    pub run_in_ci: Option<bool>,

    pub run_from_workspace_root: Option<bool>,
}

impl Default for TaskOptionsConfig {
    fn default() -> Self {
        TaskOptionsConfig {
            merge_args: Some(TaskMergeStrategy::default()),
            merge_deps: Some(TaskMergeStrategy::default()),
            merge_inputs: Some(TaskMergeStrategy::default()),
            merge_outputs: Some(TaskMergeStrategy::default()),
            retry_count: Some(0),
            run_in_ci: Some(true),
            run_from_workspace_root: Some(false),
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize, Validate)]
pub struct TaskConfig {
    pub args: Option<Vec<String>>,

    pub command: Option<String>,

    #[validate(custom = "validate_deps")]
    pub deps: Option<Vec<String>>,

    #[validate(custom = "validate_inputs")]
    pub inputs: Option<Vec<String>>,

    #[validate]
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

    mod deps {
        #[test]
        #[should_panic(
            expected = "Invalid field `deps`. Expected a sequence type, received string \"abc\"."
        )]
        fn invalid_type() {
            figment::Jail::expect_with(|jail| {
                jail.create_file(
                    super::CONFIG_FILENAME,
                    r#"
command: foo
deps: abc
"#,
                )?;

                super::load_jailed_config()?;

                Ok(())
            });
        }

        #[test]
        #[should_panic(
            expected = "Invalid field `deps.0`. Expected a string type, received unsigned int `123`."
        )]
        fn invalid_value_type() {
            figment::Jail::expect_with(|jail| {
                jail.create_file(
                    super::CONFIG_FILENAME,
                    r#"
command: foo
deps:
    - 123
"#,
                )?;

                super::load_jailed_config()?;

                Ok(())
            });
        }

        //         #[test]
        //         #[should_panic(
        //             expected = "Invalid field `deps.0`. Expected a string type, received unsigned int `123`."
        //         )]
        //         fn invalid_format() {
        //             figment::Jail::expect_with(|jail| {
        //                 jail.create_file(
        //                     super::CONFIG_FILENAME,
        //                     r#"
        // command: foo
        // deps:
        //     - foo
        // "#,
        //                 )?;

        //                 super::load_jailed_config()?;

        //                 Ok(())
        //             });
        //         }
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
        #[should_panic(expected = "Invalid field `options.mergeArgs`. Unknown option `bubble`.")]
        fn invalid_merge_strategy_type() {
            figment::Jail::expect_with(|jail| {
                jail.create_file(
                    super::CONFIG_FILENAME,
                    r#"
command: foo
options:
    mergeArgs: bubble
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
