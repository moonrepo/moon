// moon.yml

use crate::language_platform::{LanguageType, PlatformType};
use crate::portable_path::PortablePath;
use crate::project::*;
use moon_common::Id;
use rustc_hash::FxHashMap;
use schematic::{color, config_enum, Config, ConfigError, ConfigLoader, ValidateError};
use std::collections::BTreeMap;
use std::path::Path;
use strum::Display;

fn validate_channel<D, C>(value: &str, _data: &D, _ctx: &C) -> Result<(), ValidateError> {
    if !value.is_empty() && !value.starts_with('#') {
        return Err(ValidateError::new("must start with a `#`"));
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
        String(Id),
        Object { id: Id, scope: DependencyScope },
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

    pub file_groups: FxHashMap<Id, Vec<PortablePath>>,

    pub language: LanguageType,

    pub platform: Option<PlatformType>,

    #[setting(nested)]
    pub project: Option<ProjectMetadataConfig>,

    pub tags: Vec<Id>,

    #[setting(nested)]
    pub tasks: BTreeMap<Id, TaskConfig>,

    #[setting(nested)]
    pub toolchain: ProjectToolchainConfig,

    #[setting(rename = "type")]
    pub type_of: ProjectType,

    #[setting(nested)]
    pub workspace: ProjectWorkspaceConfig,
}

impl ProjectConfig {
    pub fn load<T: AsRef<Path>, F: AsRef<Path>>(
        workspace_root: T,
        path: F,
    ) -> Result<ProjectConfig, ConfigError> {
        let workspace_root = workspace_root.as_ref();
        let path = path.as_ref();

        let result = ConfigLoader::<ProjectConfig>::yaml()
            .label(color::path(path))
            .file(workspace_root.join(path))?
            .load()?;

        Ok(result.config)
    }
}
