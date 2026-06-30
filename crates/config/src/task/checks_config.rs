use crate::{config_enum, config_struct, generate_switch};
use deserialize_untagged_verbose_error::DeserializeUntaggedVerboseError;
use schematic::schema::{StringType, UnionType};
use schematic::{Config, ParseError, Schema, SchemaBuilder, Schematic, ValidateError};
use serde::{Deserialize, Serialize, Serializer};

fn check_script(script: &str) -> Result<(), String> {
    if script.trim().is_empty() {
        return Err("a shell script is required for a task check".into());
    }

    Ok(())
}

fn validate_script<D, C>(
    script: &str,
    _data: &D,
    _ctx: &C,
    _finalize: bool,
) -> Result<(), ValidateError> {
    check_script(script).map_err(ValidateError::new)
}

config_struct!(
    /// Task check configuration for conditions.
    #[derive(Config)]
    pub struct TaskCheckConditionConfig {
        /// The shell script to execute.
        #[setting(validate = validate_script)]
        pub script: String,
    }
);

config_struct!(
    /// Task check configuration for requirements.
    #[derive(Config)]
    pub struct TaskCheckRequirementConfig {
        /// The shell script to execute.
        #[setting(validate = validate_script)]
        pub script: String,
    }
);

config_enum!(
    /// The fingerprinting strategy for hashing a task check.
    #[serde(expecting = "expected `exit-code`, `stderr`, `stdout`, or a boolean")]
    pub enum TaskCheckFingerprint {
        /// Only hash the exit code.
        ExitCode,
        /// Only hash stderr.
        Stderr,
        /// Only hash stdout.
        Stdout,
        /// Whether to hash all script output.
        #[serde(untagged)]
        Enabled(bool),
    }
);

generate_switch!(TaskCheckFingerprint, ["exit-code", "stderr", "stdout"]);

impl Default for TaskCheckFingerprint {
    fn default() -> Self {
        Self::Enabled(true)
    }
}

config_struct!(
    /// Task check configuration for fingerprinting.
    #[derive(Config)]
    pub struct TaskCheckFingerprintConfig {
        /// The shell script to execute.
        #[setting(validate = validate_script)]
        pub script: String,

        /// The content hashing strategy.
        #[serde(default)]
        pub hash: TaskCheckFingerprint,
    }
);

#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
#[serde(try_from = "TaskCheckEntryShape")]
pub enum TaskCheckEntry {
    Condition(TaskCheckConditionConfig),
    Requirement(TaskCheckRequirementConfig),
    Fingerprint(TaskCheckFingerprintConfig),
}

impl Serialize for TaskCheckEntry {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Condition(config) => {
                TaggedTaskCheckEntry::Condition(config.clone()).serialize(serializer)
            }
            Self::Requirement(config) => {
                TaggedTaskCheckEntry::Requirement(config.clone()).serialize(serializer)
            }
            Self::Fingerprint(config) => {
                TaggedTaskCheckEntry::Fingerprint(config.clone()).serialize(serializer)
            }
        }
    }
}

impl Schematic for TaskCheckEntry {
    fn schema_name() -> Option<String> {
        Some("TaskCheckEntry".into())
    }

    fn build_schema(mut schema: SchemaBuilder) -> Schema {
        schema.union(UnionType::new_any([
            schema.infer::<String>(),
            schema.infer::<TaggedTaskCheckEntry>(),
        ]))
    }
}

#[derive(Config, Serialize, Deserialize)]
#[serde(tag = "check", rename_all = "kebab-case")]
enum TaggedTaskCheckEntry {
    Condition(TaskCheckConditionConfig),
    Requirement(TaskCheckRequirementConfig),
    Fingerprint(TaskCheckFingerprintConfig),
}

#[derive(DeserializeUntaggedVerboseError)]
enum TaskCheckEntryShape {
    String(String),
    Tagged(TaggedTaskCheckEntry),
}

impl TryFrom<TaskCheckEntryShape> for TaskCheckEntry {
    type Error = ParseError;

    fn try_from(shape: TaskCheckEntryShape) -> Result<Self, Self::Error> {
        match shape {
            TaskCheckEntryShape::String(script) => check_script(&script)
                .map(|_| Self::Requirement(TaskCheckRequirementConfig { script }))
                .map_err(ParseError::new),
            TaskCheckEntryShape::Tagged(tagged) => match tagged {
                TaggedTaskCheckEntry::Requirement(config) => Ok(Self::Requirement(config)),
                TaggedTaskCheckEntry::Condition(config) => Ok(Self::Condition(config)),
                TaggedTaskCheckEntry::Fingerprint(config) => Ok(Self::Fingerprint(config)),
            },
        }
    }
}
