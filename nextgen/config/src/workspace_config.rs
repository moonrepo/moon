// .moon/workspace.yml

use crate::validate::{check_map, validate_child_relative_path, validate_semver_requirement};
use crate::workspace::*;
use moon_common::Id;
use rustc_hash::FxHashMap;
use schematic::{config_enum, validate, Config, ConfigError, ConfigLoader, ValidateError};
use std::path::Path;

type ProjectsMap = FxHashMap<Id, String>;

// Validate the `projects` field is a map of valid file system paths
// that are relative from the workspace root. Will fail on absolute
// paths ("/"), and parent relative paths ("../").
fn validate_projects(projects: &WorkspaceProjects) -> Result<(), ValidateError> {
    let map = match projects {
        WorkspaceProjects::Sources(sources) => sources,
        WorkspaceProjects::Both { sources, .. } => sources,
        _ => return Ok(()),
    };

    check_map(map, |value| validate_child_relative_path(value))?;

    Ok(())
}

config_enum!(
    #[serde(
        untagged,
        expecting = "expected a sequence of globs or a map of projects"
    )]
    pub enum WorkspaceProjects {
        Both {
            globs: Vec<String>,
            sources: ProjectsMap,
        },
        Globs(Vec<String>),
        Sources(ProjectsMap),
    }
);

impl Default for WorkspaceProjects {
    fn default() -> Self {
        WorkspaceProjects::Sources(FxHashMap::default())
    }
}

#[derive(Config)]
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

    #[setting(validate = validate_projects)]
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
