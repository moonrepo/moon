// .moon/workspace.yml

use crate::portable_path::{Portable, ProjectFilePath, ProjectGlobPath};
use crate::validate::validate_semver_requirement;
use crate::workspace::*;
use moon_common::{consts, Id};
use rustc_hash::FxHashMap;
use schematic::{
    derive_enum, validate, Config, ConfigError, ConfigLoader, Segment, SettingPath, ValidateError,
};
use std::path::Path;

// We can't use serde based types in the enum below to handle validation,
// as serde fails to parse correctly. So we must manually validate here.
fn validate_projects<D, C>(
    projects: &WorkspaceProjects,
    _data: &D,
    _ctx: &C,
) -> Result<(), ValidateError> {
    match projects {
        WorkspaceProjects::Both { globs, sources } => {
            for (i, g) in globs.iter().enumerate() {
                ProjectGlobPath::from_str(g).map_err(|mut error| {
                    error.path =
                        SettingPath::new(vec![Segment::Key("globs".to_owned()), Segment::Index(i)]);
                    error
                })?;
            }

            for (k, v) in sources {
                ProjectFilePath::from_str(v).map_err(|mut error| {
                    error.path = SettingPath::new(vec![
                        Segment::Key("sources".to_owned()),
                        Segment::Key(k.to_string()),
                    ]);
                    error
                })?;
            }
        }
        WorkspaceProjects::Globs(globs) => {
            for (i, g) in globs.iter().enumerate() {
                ProjectGlobPath::from_str(g).map_err(|mut error| {
                    error.path = SettingPath::new(vec![Segment::Index(i)]);
                    error
                })?;
            }
        }
        WorkspaceProjects::Sources(sources) => {
            for (k, v) in sources {
                ProjectFilePath::from_str(v).map_err(|mut error| {
                    error.path = SettingPath::new(vec![Segment::Key(k.to_string())]);
                    error
                })?;
            }
        }
    };

    Ok(())
}

derive_enum!(
    #[serde(
        untagged,
        expecting = "expected a sequence of globs or a map of projects"
    )]
    pub enum WorkspaceProjects {
        Both {
            globs: Vec<String>,
            sources: FxHashMap<Id, String>,
        },
        Globs(Vec<String>),
        Sources(FxHashMap<Id, String>),
    }
);

impl Default for WorkspaceProjects {
    fn default() -> Self {
        WorkspaceProjects::Sources(FxHashMap::default())
    }
}

/// Docs: https://moonrepo.dev/docs/config/workspace
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
    pub fn load<R: AsRef<Path>, P: AsRef<Path>>(
        workspace_root: R,
        path: P,
    ) -> Result<WorkspaceConfig, ConfigError> {
        let result = ConfigLoader::<WorkspaceConfig>::new()
            .set_root(workspace_root.as_ref())
            .file(path.as_ref())?
            .load()?;

        Ok(result.config)
    }

    pub fn load_from<P: AsRef<Path>>(workspace_root: P) -> Result<WorkspaceConfig, ConfigError> {
        let workspace_root = workspace_root.as_ref();

        Self::load(
            workspace_root,
            workspace_root
                .join(consts::CONFIG_DIRNAME)
                .join(consts::CONFIG_WORKSPACE_FILENAME),
        )
    }
}
