use crate::types::{FilePath, FilePathOrGlob, TargetID};
use crate::validators::{validate_child_or_root_path, validate_target};
use schemars::gen::SchemaGenerator;
use schemars::schema::Schema;
use schemars::{schema_for, JsonSchema};
use serde::de::{self, SeqAccess};
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;
use std::fmt;
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

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskType {
    Node,
    System,
}

impl Default for TaskType {
    fn default() -> Self {
        TaskType::Node
    }
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
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

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct TaskOptionsConfig {
    pub merge_args: Option<TaskMergeStrategy>,

    pub merge_deps: Option<TaskMergeStrategy>,

    pub merge_env: Option<TaskMergeStrategy>,

    pub merge_inputs: Option<TaskMergeStrategy>,

    pub merge_outputs: Option<TaskMergeStrategy>,

    pub retry_count: Option<u8>,

    #[serde(rename = "runInCI")]
    pub run_in_ci: Option<bool>,

    pub run_from_workspace_root: Option<bool>,
}

impl Default for TaskOptionsConfig {
    fn default() -> Self {
        TaskOptionsConfig {
            merge_args: Some(TaskMergeStrategy::default()),
            merge_deps: Some(TaskMergeStrategy::default()),
            merge_env: Some(TaskMergeStrategy::default()),
            merge_inputs: Some(TaskMergeStrategy::default()),
            merge_outputs: Some(TaskMergeStrategy::default()),
            retry_count: Some(0),
            run_in_ci: Some(true),
            run_from_workspace_root: Some(false),
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, JsonSchema, PartialEq, Serialize, Validate)]
pub struct TaskConfig {
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_args")]
    #[schemars(schema_with = "make_args_schema")]
    pub args: Option<Vec<String>>,

    pub command: Option<String>,

    #[validate(custom = "validate_deps")]
    pub deps: Option<Vec<TargetID>>,

    pub env: Option<HashMap<String, String>>,

    #[validate(custom = "validate_inputs")]
    pub inputs: Option<Vec<FilePathOrGlob>>,

    #[serde(default)]
    #[validate]
    pub options: TaskOptionsConfig,

    #[validate(custom = "validate_outputs")]
    pub outputs: Option<Vec<FilePath>>,

    #[serde(default)]
    #[serde(rename = "type")]
    pub type_of: TaskType,
}

// SERDE

#[derive(JsonSchema)]
#[serde(untagged)]
enum Args {
    #[allow(dead_code)]
    Str(String),
    #[allow(dead_code)]
    Vec(Vec<String>),
}

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
        match shell_words::split(value) {
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

fn make_args_schema(_gen: &mut SchemaGenerator) -> Schema {
    let root = schema_for!(Args);

    Schema::Object(root.schema)

    // let mut schema: SchemaObject = <String>::json_schema(gen).into();
    // schema.instance_type = None;
    // schema.subschemas = Some(Box::new(SubschemaValidation {
    //     one_of: Some(vec![
    //         Schema::Object(SchemaObject {
    //             instance_type: Some(SingleOrVec::Single(Box::new(InstanceType::String))),
    //             ..SchemaObject::default()
    //         }),
    //         Schema::Object(SchemaObject {
    //             instance_type: Some(SingleOrVec::Single(Box::new(InstanceType::Array))),
    //             array: Some(Box::new(ArrayValidation {
    //                 items: Some(SingleOrVec::Single(Box::new(Schema::Object(
    //                     SchemaObject {
    //                         instance_type: Some(SingleOrVec::Single(Box::new(
    //                             InstanceType::String,
    //                         ))),
    //                         ..SchemaObject::default()
    //                     },
    //                 )))),
    //                 ..ArrayValidation::default()
    //             })),
    //             ..SchemaObject::default()
    //         }),
    //     ]),
    //     ..SubschemaValidation::default()
    // }));
    // schema.into()
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
        use super::TaskConfig;
        use moon_utils::string_vec;

        #[test]
        #[should_panic(
            expected = "Invalid field `args`. Expected a sequence of strings or a string type, received unsigned int `123`."
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

    mod env {
        #[test]
        #[should_panic(
            expected = "Invalid field `env`. Expected a map type, received string \"abc\"."
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
            expected = "Invalid field `env.KEY`. Expected a string type, received unsigned int `123`."
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
