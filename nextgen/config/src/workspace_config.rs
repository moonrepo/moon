// .moon/workspace.yml

use crate::relative_path::{FilePath, GlobPath, ProjectRelativePath};
use crate::validate::validate_semver_requirement;
use crate::workspace::*;
use moon_common::Id;
use rustc_hash::FxHashMap;
use schematic::{config_enum, validate, Config, ConfigError, ConfigLoader};
use std::path::Path;

type SourceGlob = ProjectRelativePath<GlobPath>;
type SourceFile = ProjectRelativePath<FilePath>;

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
}
