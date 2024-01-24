// .moon/workspace.yml

use crate::portable_path::{Portable, ProjectFilePath, ProjectGlobPath};
use crate::validate::check_yml_extension;
use crate::workspace::*;
use moon_common::{color, consts, Id};
use proto_core::VersionReq;
use rustc_hash::FxHashMap;
use schematic::{validate, Config, ConfigLoader, Path as SettingPath, PathSegment, ValidateError};
use std::path::Path;

// We can't use serde based types in the enum below to handle validation,
// as serde fails to parse correctly. So we must manually validate here.
fn validate_projects<D, C>(
    projects: &PartialWorkspaceProjects,
    _data: &D,
    _ctx: &C,
) -> Result<(), ValidateError> {
    match projects {
        PartialWorkspaceProjects::Both(cfg) => {
            if let Some(globs) = &cfg.globs {
                for (i, g) in globs.iter().enumerate() {
                    ProjectGlobPath::from_str(g).map_err(|mut error| {
                        error.path = SettingPath::new(vec![
                            PathSegment::Key("globs".to_owned()),
                            PathSegment::Index(i),
                        ]);
                        error
                    })?;
                }
            }

            if let Some(sources) = &cfg.sources {
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
        }
        PartialWorkspaceProjects::Globs(globs) => {
            for (i, g) in globs.iter().enumerate() {
                ProjectGlobPath::from_str(g).map_err(|mut error| {
                    error.path = SettingPath::new(vec![PathSegment::Index(i)]);
                    error
                })?;
            }
        }
        PartialWorkspaceProjects::Sources(sources) => {
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

#[derive(Clone, Config, Debug)]
pub struct WorkspaceProjectsConfig {
    pub globs: Vec<String>,
    pub sources: FxHashMap<Id, String>,
}

#[derive(Clone, Config, Debug)]
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
#[derive(Clone, Config, Debug)]
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

    #[setting(nested)]
    pub experiments: ExperimentsConfig,

    #[setting(extend, validate = validate::extends_string)]
    pub extends: Option<String>,

    #[setting(nested)]
    pub extensions: FxHashMap<proto_core::Id, ExtensionConfig>,

    #[setting(nested)]
    pub generator: GeneratorConfig,

    #[setting(nested)]
    pub hasher: HasherConfig,

    #[setting(nested)]
    pub notifier: NotifierConfig,

    #[setting(nested, validate = validate_projects)]
    pub projects: WorkspaceProjects,

    #[setting(nested)]
    pub runner: RunnerConfig,

    #[setting(default = true)]
    pub telemetry: bool,

    #[setting(nested)]
    pub vcs: VcsConfig,

    pub version_constraint: Option<VersionReq>,
}

impl WorkspaceConfig {
    pub fn load<R: AsRef<Path>, P: AsRef<Path>>(
        workspace_root: R,
        path: P,
    ) -> miette::Result<WorkspaceConfig> {
        let result = ConfigLoader::<WorkspaceConfig>::new()
            .set_help(color::muted_light(
                "https://moonrepo.dev/docs/config/workspace",
            ))
            .set_root(workspace_root.as_ref())
            .file(check_yml_extension(path.as_ref()))?
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
