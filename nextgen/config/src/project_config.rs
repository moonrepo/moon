// moon.yml

use crate::language_platform::{LanguageType, PlatformType};
use crate::project::*;
use crate::relative_path::RelativePath;
use moon_common::Id;
use rustc_hash::FxHashMap;
use schematic::{config_enum, Config, ConfigError, ConfigLoader, Segment, ValidateError};
use std::collections::BTreeMap;
use std::path::Path;
use strum::Display;

fn validate_channel(value: &str) -> Result<(), ValidateError> {
    if !value.is_empty() && !value.starts_with('#') {
        return Err(ValidateError::new("must start with a `#`"));
    }

    Ok(())
}

// TODO
fn validate_tasks(map: &BTreeMap<String, TaskConfig>) -> Result<(), ValidateError> {
    for (name, task) in map {
        // Only fail for empty strings and not `None`
        if task.command.is_some() && task.get_command().is_empty() {
            return Err(ValidateError::with_segments(
                "a command is required; use \"noop\" otherwise",
                vec![Segment::Key(name.to_string())],
            ));
        }
    }

    Ok(())
}

config_enum!(
    #[derive(Default, Display)]
    pub enum ProjectType {
        #[strum(serialize = "application")]
        Application,

        #[strum(serialize = "library")]
        Library,

        #[strum(serialize = "tool")]
        Tool,

        #[default]
        #[strum(serialize = "unknown")]
        Unknown,
    }
);

#[derive(Config)]
pub struct ProjectMetadataConfig {
    pub name: Option<String>,

    pub description: String,

    pub owner: Option<String>,

    pub maintainers: Vec<String>,

    #[setting(validate = validate_channel)]
    pub channel: Option<String>,
}

config_enum!(
    #[serde(
        untagged,
        expecting = "expected a project name or dependency config object"
    )]
    pub enum ProjectDependsOn {
        String(String),
        Object { id: String, scope: DependencyScope },
    }
);

/// Docs: https://moonrepo.dev/docs/config/project
#[derive(Config)]
pub struct ProjectConfig {
    #[setting(
        default = "https://moonrepo.dev/schemas/project.json",
        rename = "$schema"
    )]
    pub schema: String,

    pub depends_on: Vec<ProjectDependsOn>,

    pub env: FxHashMap<String, String>,

    pub file_groups: FxHashMap<Id, Vec<RelativePath>>,

    pub language: LanguageType,

    pub platform: Option<PlatformType>,

    #[setting(nested)]
    pub project: Option<ProjectMetadataConfig>,

    pub tags: Vec<Id>,

    // TODO
    // #[setting(nested)]
    // pub tasks: BTreeMap<Id, TaskConfig>,
    #[setting(nested)]
    pub toolchain: ProjectToolchainConfig,

    #[setting(rename = "type")]
    pub type_of: ProjectType,

    #[setting(nested)]
    pub workspace: ProjectWorkspaceConfig,
}

impl ProjectConfig {
    pub fn load<T: AsRef<Path>>(path: T) -> Result<ProjectConfig, ConfigError> {
        let result = ConfigLoader::<ProjectConfig>::yaml()
            .file(path.as_ref())?
            .load()?;

        Ok(result.config)
    }
}
