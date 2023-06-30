// .moon/workspace.yml

use crate::portable_path::{Portable, ProjectFilePath, ProjectGlobPath};
use crate::validate::validate_semver_requirement;
use crate::workspace::*;
use moon_common::{consts, Id};
use rustc_hash::FxHashMap;
use schematic::{validate, Config, ConfigLoader, Path as SettingPath, PathSegment, ValidateError};
use std::path::Path;

// TODO
// We can't use serde based types in the enum below to handle validation,
// as serde fails to parse correctly. So we must manually validate here.
fn validate_projects<D, C>(
    projects: &WorkspaceProjects,
    _data: &D,
    _ctx: &C,
) -> Result<(), ValidateError> {
    match projects {
        WorkspaceProjects::Both(WorkspaceProjectsConfig { globs, sources }) => {
            for (i, g) in globs.iter().enumerate() {
                ProjectGlobPath::from_str(g).map_err(|mut error| {
                    error.path = SettingPath::new(vec![
                        PathSegment::Key("globs".to_owned()),
                        PathSegment::Index(i),
                    ]);
                    error
                })?;
            }

            for (k, v) in sources {
                ProjectFilePath::from_str(v).map_err(|mut error| {
                    error.path = SettingPath::new(vec![
                        PathSegment::Key("sources".to_owned()),
                        PathSegment::Key(k.to_string()),
                    ]);
                    error
                })?;
            }
        }
        WorkspaceProjects::Globs(globs) => {
            for (i, g) in globs.iter().enumerate() {
                ProjectGlobPath::from_str(g).map_err(|mut error| {
                    error.path = SettingPath::new(vec![PathSegment::Index(i)]);
                    error
                })?;
            }
        }
        WorkspaceProjects::Sources(sources) => {
            for (k, v) in sources {
                ProjectFilePath::from_str(v).map_err(|mut error| {
                    error.path = SettingPath::new(vec![PathSegment::Key(k.to_string())]);
                    error
                })?;
            }
        }
    };

    Ok(())
}

#[derive(Config, Debug)]
pub struct WorkspaceProjectsConfig {
    pub globs: Vec<String>,
    pub sources: FxHashMap<Id, String>,
}

#[derive(Config, Debug)]
#[config(serde(
    untagged,
    expecting = "expected a list of globs, a map of projects, or both"
))]
pub enum WorkspaceProjects {
    #[setting(nested)]
    Both(WorkspaceProjectsConfig),

    Globs(Vec<String>),

    #[setting(default)]
    Sources(FxHashMap<Id, String>),
}

/// Docs: https://moonrepo.dev/docs/config/workspace
#[derive(Config, Debug)]
pub struct WorkspaceConfig {
    #[setting(
        default = "https://moonrepo.dev/schemas/workspace.json",
        rename = "$schema"
    )]
    pub schema: String,

    #[setting(nested)]
    pub codeowners: CodeownersConfig,

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

    #[setting(nested)]
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
    pub fn load<R: AsRef<Path>, P: AsRef<Path>>(
        workspace_root: R,
        path: P,
    ) -> miette::Result<WorkspaceConfig> {
        let result = ConfigLoader::<WorkspaceConfig>::new()
            .set_root(workspace_root.as_ref())
            .file(path.as_ref())?
            .load()?;

        Ok(result.config)
    }

    pub fn load_from<P: AsRef<Path>>(workspace_root: P) -> miette::Result<WorkspaceConfig> {
        let workspace_root = workspace_root.as_ref();

        Self::load(
            workspace_root,
            workspace_root
                .join(consts::CONFIG_DIRNAME)
                .join(consts::CONFIG_WORKSPACE_FILENAME),
        )
    }
}
