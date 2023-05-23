// .moon/workspace.yml

use crate::portable_path::{FilePath, GlobPath, ProjectPortablePath};
use crate::validate::validate_semver_requirement;
use crate::workspace::*;
use moon_common::{consts, Id};
use rustc_hash::FxHashMap;
use schematic::{config_enum, validate, Config, ConfigError, ConfigLoader};
use std::path::Path;

type SourceGlob = ProjectPortablePath<GlobPath>;
type SourceFile = ProjectPortablePath<FilePath>;

config_enum!(
    #[serde(
        untagged,
        expecting = "expected a sequence of globs or a map of projects"
    )]
    pub enum WorkspaceProjects {
        Both {
            globs: Vec<SourceGlob>,
            sources: FxHashMap<Id, SourceFile>,
        },
        Globs(Vec<SourceGlob>),
        Sources(FxHashMap<Id, SourceFile>),
    }
);

impl Default for WorkspaceProjects {
    fn default() -> Self {
        WorkspaceProjects::Sources(FxHashMap::default())
    }
}

#[derive(Config)]
#[config(file = ".moon/workspace.yml")]
pub struct WorkspaceConfig {
    #[setting(
        default = "https://moonrepo.dev/schemas/workspace.json",
        rename = "$schema"
    )]
    pub schema: String,

    #[setting(nested)]
    pub constraints: ConstraintsConfig,

    #[setting(extend, validate = validate::extends_string)]
    pub extends: Option<String>,

    #[setting(nested)]
    pub generator: GeneratorConfig,

    #[setting(nested)]
    pub hasher: HasherConfig,

    #[setting(nested)]
    pub notifier: NotifierConfig,

    pub projects: WorkspaceProjects,

    #[setting(nested)]
    pub runner: RunnerConfig,

    #[setting(default = true)]
    pub telemetry: bool,

    #[setting(nested)]
    pub vcs: VcsConfig,

    #[setting(validate = validate_semver_requirement)]
    pub version_constraint: Option<String>,
}

impl WorkspaceConfig {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<WorkspaceConfig, ConfigError> {
        let result = ConfigLoader::<WorkspaceConfig>::yaml()
            .file(path.as_ref())?
            .load()?;

        Ok(result.config)
    }

    pub fn load_from<P: AsRef<Path>>(workspace_root: P) -> Result<WorkspaceConfig, ConfigError> {
        Self::load(
            workspace_root
                .as_ref()
                .join(consts::CONFIG_DIRNAME)
                .join(consts::CONFIG_WORKSPACE_FILENAME),
        )
    }
}
