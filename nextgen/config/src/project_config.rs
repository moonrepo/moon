// moon.yml

use crate::language_platform::{LanguageType, PlatformType};
use crate::portable_path::PortablePath;
use crate::project::*;
use moon_common::{consts, Id};
use rustc_hash::FxHashMap;
use schematic::{color, config_enum, validate, Config, ConfigError, ConfigLoader, ValidateError};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;
use strum::{Display, EnumString};

fn validate_channel<D, C>(value: &str, _data: &D, _ctx: &C) -> Result<(), ValidateError> {
    if !value.is_empty() && !value.starts_with('#') {
        return Err(ValidateError::new("must start with a `#`"));
    }

    Ok(())
}

config_enum!(
    #[derive(Copy, Default, Display, EnumString)]
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

#[derive(Clone, Config, Deserialize, Serialize)]
pub struct ProjectMetadataConfig {
    pub name: Option<String>,

    #[setting(validate = validate::not_empty)]
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
#[derive(Clone, Config, Deserialize, Serialize)]
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
    pub fn load<R: AsRef<Path>, P: AsRef<Path>>(
        workspace_root: R,
        path: P,
    ) -> Result<ProjectConfig, ConfigError> {
        let workspace_root = workspace_root.as_ref();
        let path = path.as_ref();

        let mut loader = ConfigLoader::<ProjectConfig>::yaml();
        loader.label(color::path(path.strip_prefix(workspace_root).unwrap()));

        if path.exists() {
            loader.file(path)?;
        }

        let result = loader.load()?;

        Ok(result.config)
    }

    pub fn load_from<R: AsRef<Path>, P: AsRef<str>>(
        workspace_root: R,
        project_source: P,
    ) -> Result<ProjectConfig, ConfigError> {
        let workspace_root = workspace_root.as_ref();

        Self::load(
            workspace_root,
            workspace_root
                .join(project_source.as_ref())
                .join(consts::CONFIG_PROJECT_FILENAME),
        )
    }
}
