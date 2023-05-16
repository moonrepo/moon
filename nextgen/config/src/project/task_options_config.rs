use crate::relative_path::RelativePath;
use schematic::{config_enum, Config, ValidateError};

fn validate_affected_files<D, C>(
    file: &TaskOptionAffectedFiles,
    _data: &D,
    _ctx: &C,
) -> Result<(), ValidateError> {
    if let TaskOptionAffectedFiles::Value(value) = file {
        if value != "args" && value != "env" {
            return Err(ValidateError::new("expected `args`, `env`, or a boolean"));
        }
    }

    Ok(())
}

config_enum!(
    #[serde(untagged, expecting = "expected `args`, `env`, or a boolean")]
    pub enum TaskOptionAffectedFiles {
        Enabled(bool),
        Value(String),
    }
);

config_enum!(
    #[serde(untagged, expecting = "expected a boolean or a file system path")]
    pub enum TaskOptionEnvFile {
        Enabled(bool),
        File(RelativePath),
    }
);

config_enum!(
    #[derive(Default)]
    pub enum TaskMergeStrategy {
        #[default]
        Append,
        Prepend,
        Replace,
    }
);

config_enum!(
    #[derive(Default)]
    pub enum TaskOutputStyle {
        #[default]
        Buffer,
        BufferOnlyFailure,
        Hash,
        None,
        Stream,
    }
);

#[derive(Debug, Clone, Config)]
pub struct TaskOptionsConfig {
    #[setting(validate = validate_affected_files)]
    pub affected_files: Option<TaskOptionAffectedFiles>,

    #[setting(default = true)]
    pub cache: bool,

    pub env_file: Option<TaskOptionEnvFile>,

    pub merge_args: TaskMergeStrategy,

    pub merge_deps: TaskMergeStrategy,

    pub merge_env: TaskMergeStrategy,

    pub merge_inputs: TaskMergeStrategy,

    pub merge_outputs: TaskMergeStrategy,

    pub output_style: TaskOutputStyle,

    pub persistent: bool,

    pub retry_count: u8,

    #[setting(default = true)]
    pub run_deps_in_parallel: bool,

    #[setting(default = true, rename = "runInCI")]
    pub run_in_ci: bool,

    pub run_from_workspace_root: bool,

    #[setting(default = true)]
    pub shell: bool,
}
