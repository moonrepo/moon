use crate::project::{ProjectConfig, ProjectLanguage};
use crate::types::{FilePath, InputValue, TargetID};
use crate::validators::{skip_if_default, validate_child_or_root_path, validate_target};
use moon_utils::process::split_args;
use moon_utils::regex::ENV_VAR;
use schemars::gen::SchemaGenerator;
use schemars::schema::Schema;
use schemars::{schema_for, JsonSchema};
use serde::de::{self, SeqAccess};
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;
use std::fmt;
use strum::Display;
use validator::{Validate, ValidationError};

// These structs utilize optional fields so that we can handle merging effectively,
// as we need a way to skip "undefined" values. So don't use serde defaults here.

fn validate_deps(list: &[String]) -> Result<(), ValidationError> {
    for (index, item) in list.iter().enumerate() {
        validate_target(&format!("deps[{}]", index), item)?;
    }

    Ok(())
}

fn validate_inputs(list: &[String]) -> Result<(), ValidationError> {
    for (index, item) in list.iter().enumerate() {
        if !ENV_VAR.is_match(item) {
            validate_child_or_root_path(&format!("inputs[{}]", index), item)?;
        }
    }

    Ok(())
}

fn validate_outputs(list: &[String]) -> Result<(), ValidationError> {
    for (index, item) in list.iter().enumerate() {
        validate_child_or_root_path(&format!("outputs[{}]", index), item)?;
    }

    Ok(())
}

#[derive(Clone, Debug, Default, Deserialize, Display, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PlatformType {
    #[strum(serialize = "node")]
    Node,

    #[strum(serialize = "system")]
    System,

    #[default]
    #[strum(serialize = "unknown")]
    Unknown,
}

#[derive(Clone, Debug, Default, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskMergeStrategy {
    #[default]
    Append,
    Prepend,
    Replace,
}

#[derive(Clone, Debug, Default, Deserialize, JsonSchema, PartialEq, Serialize, Validate)]
#[serde(default, rename_all = "camelCase")]
pub struct TaskOptionsConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub merge_args: Option<TaskMergeStrategy>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub merge_deps: Option<TaskMergeStrategy>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub merge_env: Option<TaskMergeStrategy>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub merge_inputs: Option<TaskMergeStrategy>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub merge_outputs: Option<TaskMergeStrategy>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_count: Option<u8>,

    #[serde(rename = "runInCI", skip_serializing_if = "Option::is_none")]
    pub run_in_ci: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_from_workspace_root: Option<bool>,

    #[serde(rename = "streamOutput", skip_serializing_if = "Option::is_none")]
    pub stream_output: Option<bool>,
}

// We use serde(default) here because figment *does not* apply defaults
// for structs nested within collections. Primarily hash maps.
#[derive(Clone, Debug, Default, Deserialize, JsonSchema, PartialEq, Serialize, Validate)]
#[serde(default)]
pub struct TaskConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,

    #[serde(
        deserialize_with = "deserialize_args",
        skip_serializing_if = "Option::is_none"
    )]
    #[schemars(schema_with = "make_args_schema")]
    pub args: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(custom = "validate_deps")]
    pub deps: Option<Vec<TargetID>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<HashMap<String, String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(custom = "validate_inputs")]
    pub inputs: Option<Vec<InputValue>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(custom = "validate_outputs")]
    pub outputs: Option<Vec<FilePath>>,

    #[serde(skip_serializing_if = "skip_if_default")]
    #[validate]
    pub options: TaskOptionsConfig,

    #[serde(skip_serializing_if = "skip_if_default")]
    #[serde(rename = "type")]
    pub type_of: PlatformType,
}

impl TaskConfig {
    pub fn detect_platform(project: &ProjectConfig) -> PlatformType {
        match &project.language {
            ProjectLanguage::JavaScript | ProjectLanguage::TypeScript => PlatformType::Node,
            ProjectLanguage::Bash | ProjectLanguage::Batch => PlatformType::System,
            _ => PlatformType::Unknown,
        }
    }
}

// SERDE

struct DeserializeArgs;

impl<'de> de::Visitor<'de> for DeserializeArgs {
    type Value = Vec<String>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a sequence of strings or a string")
    }

    fn visit_seq<V>(self, mut visitor: V) -> Result<Self::Value, V::Error>
    where
        V: SeqAccess<'de>,
    {
        let mut vec = Vec::new();

        while let Some(elem) = visitor.next_element()? {
            vec.push(elem);
        }

        Ok(vec)
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        match split_args(value) {
            Ok(args) => Ok(args),
            Err(error) => Err(E::custom(error)),
        }
    }
}

fn deserialize_args<'de, D>(deserializer: D) -> Result<Option<Vec<String>>, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(Some(deserializer.deserialize_any(DeserializeArgs)?))
}

// JSON SCHEMA

#[derive(JsonSchema)]
#[serde(untagged)]
enum ArgsField {
    #[allow(dead_code)]
    String(String),
    #[allow(dead_code)]
    Sequence(Vec<String>),
}

fn make_args_schema(_gen: &mut SchemaGenerator) -> Schema {
    let root = schema_for!(ArgsField);

    Schema::Object(root.schema)
}

#[cfg(test)]
mod tests {
    use super::*;
    use figment::{
        providers::{Format, Yaml},
        Figment,
    };
    use std::path::PathBuf;

    const CONFIG_FILENAME: &str = "tasks.yml";

    // Not a config file, but we want to test in isolation
    fn load_jailed_config() -> Result<TaskConfig, figment::Error> {
        Figment::new()
            .merge(Yaml::file(&PathBuf::from(CONFIG_FILENAME)))
            .extract()
    }

    mod command {
        #[test]
        #[should_panic(
            expected = "invalid type: found unsigned int `123`, expected a string for key \"default.command\""
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
        use super::TaskConfig;
        use moon_utils::string_vec;

        #[test]
        #[should_panic(
            expected = "invalid type: found unsigned int `123`, expected a sequence of strings or a string for key \"default.args\""
        )]
        fn invalid_type() {
            figment::Jail::expect_with(|jail| {
                jail.create_file(
                    super::CONFIG_FILENAME,
                    r#"
command: foo
args: 123
"#,
                )?;

                super::load_jailed_config()?;

                Ok(())
            });
        }

        #[test]
        #[should_panic(
            expected = "invalid type: found unsigned int `123`, expected a string for key \"default.args.0\""
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

        #[test]
        fn supports_vec_strings() {
            figment::Jail::expect_with(|jail| {
                jail.create_file(
                    super::CONFIG_FILENAME,
                    r#"
command: foo
args:
    - arg
    - -o
    - '@token(0)'
    - --opt
    - value
    - 'quoted arg'
"#,
                )?;

                let config = super::load_jailed_config()?;

                assert_eq!(
                    config,
                    TaskConfig {
                        command: Some(String::from("foo")),
                        args: Some(string_vec![
                            "arg",
                            "-o",
                            "@token(0)",
                            "--opt",
                            "value",
                            "quoted arg"
                        ]),
                        ..TaskConfig::default()
                    }
                );

                Ok(())
            });
        }

        #[test]
        fn supports_string() {
            figment::Jail::expect_with(|jail| {
                jail.create_file(
                    super::CONFIG_FILENAME,
                    r#"
command: foo
args: 'arg -o @token(0) --opt value "quoted arg"'
"#,
                )?;

                let config = super::load_jailed_config()?;

                assert_eq!(
                    config,
                    TaskConfig {
                        command: Some(String::from("foo")),
                        args: Some(string_vec![
                            "arg",
                            "-o",
                            "@token(0)",
                            "--opt",
                            "value",
                            "quoted arg"
                        ]),
                        ..TaskConfig::default()
                    }
                );

                Ok(())
            });
        }
    }

    mod deps {
        #[test]
        #[should_panic(
            expected = "invalid type: found string \"abc\", expected a sequence for key \"default.deps\""
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
            expected = "invalid type: found unsigned int `123`, expected a string for key \"default.deps.0\""
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
        //             expected = "Invalid field <id>deps.0</id>: Expected a string type, received unsigned int `123`."
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

    mod env {
        #[test]
        #[should_panic(
            expected = "invalid type: found string \"abc\", expected a map for key \"default.env\""
        )]
        fn invalid_type() {
            figment::Jail::expect_with(|jail| {
                jail.create_file(
                    super::CONFIG_FILENAME,
                    r#"
command: foo
env: abc
"#,
                )?;

                super::load_jailed_config()?;

                Ok(())
            });
        }

        #[test]
        #[should_panic(
            expected = "invalid type: found unsigned int `123`, expected a string for key \"default.env.KEY\""
        )]
        fn invalid_value_type() {
            figment::Jail::expect_with(|jail| {
                jail.create_file(
                    super::CONFIG_FILENAME,
                    r#"
command: foo
env:
  KEY: 123
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
            expected = "invalid type: found string \"abc\", expected a sequence for key \"default.inputs\""
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
            expected = "invalid type: found unsigned int `123`, expected a string for key \"default.inputs.0\""
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

        #[test]
        fn supports_env_vars() {
            figment::Jail::expect_with(|jail| {
                jail.create_file(
                    super::CONFIG_FILENAME,
                    r#"
command: foo
inputs:
  - $FOO
  - file.js
  - /file.js
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
            expected = "invalid type: found string \"abc\", expected a sequence for key \"default.outputs\""
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
            expected = "invalid type: found unsigned int `123`, expected a string for key \"default.outputs.0\""
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
        #[should_panic(
            expected = "unknown variant: found `whatisthis`, expected `one of `node`, `system`, `unknown`` for key \"default.type\""
        )]
        fn invalid_type() {
            figment::Jail::expect_with(|jail| {
                jail.create_file(
                    super::CONFIG_FILENAME,
                    r#"
command: foo
type: whatisthis
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
            expected = "invalid type: found unsigned int `123`, expected struct TaskOptionsConfig for key \"default.options\""
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
            expected = "unknown variant: found `bubble`, expected `one of `append`, `prepend`, `replace`` for key \"default.options.mergeArgs\""
        )]
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
            expected = "invalid type: found string \"abc\", expected u8 for key \"default.options.retryCount\""
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
