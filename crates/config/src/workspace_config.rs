use crate::portable_path::{PortablePath, ProjectFilePath, ProjectGlobPath};
use crate::workspace::*;
use moon_common::Id;
use rustc_hash::FxHashMap;
use schematic::{validate, Config, PathSegment, ValidateError};
use semver::VersionReq;

#[cfg(feature = "loader")]
use std::path::Path;

// We can't use serde based types in the enum below to handle validation,
// as serde fails to parse correctly. So we must manually validate here.
fn validate_projects<D, C>(
    projects: &PartialWorkspaceProjects,
    _data: &D,
    _ctx: &C,
    _finalize: bool,
) -> Result<(), ValidateError> {
    match projects {
        PartialWorkspaceProjects::Both(cfg) => {
            if let Some(globs) = &cfg.globs {
                for (i, g) in globs.iter().enumerate() {
                    ProjectGlobPath::from_str(g).map_err(|error| {
                        ValidateError::with_segments(
                            error.to_string(),
                            [PathSegment::Key("globs".to_owned()), PathSegment::Index(i)],
                        )
                    })?;
                }
            }

            if let Some(sources) = &cfg.sources {
                for (k, v) in sources {
                    ProjectFilePath::from_str(v).map_err(|error| {
                        ValidateError::with_segments(
                            error.to_string(),
                            [
                                PathSegment::Key("sources".to_owned()),
                                PathSegment::Key(k.to_string()),
                            ],
                        )
                    })?;
                }
            }
        }
        PartialWorkspaceProjects::Globs(globs) => {
            for (i, g) in globs.iter().enumerate() {
                ProjectGlobPath::from_str(g).map_err(|error| {
                    ValidateError::with_segments(error.to_string(), [PathSegment::Index(i)])
                })?;
            }
        }
        PartialWorkspaceProjects::Sources(sources) => {
            for (k, v) in sources {
                ProjectFilePath::from_str(v).map_err(|error| {
                    ValidateError::with_segments(
                        error.to_string(),
                        [PathSegment::Key(k.to_string())],
                    )
                })?;
            }
        }
    };

    Ok(())
}

/// Configures projects in the workspace, using both globs and explicit source paths.
#[derive(Clone, Config, Debug)]
pub struct WorkspaceProjectsConfig {
    /// A list of globs in which to locate project directories.
    /// Can be suffixed with `moon.yml` or `moon.pkl` to only find distinct projects.
    pub globs: Vec<String>,

    /// A mapping of project IDs to relative file paths to each project directory.
    pub sources: FxHashMap<Id, String>,
}

/// Configures projects in the workspace.
#[derive(Clone, Config, Debug)]
#[config(serde(
    untagged,
    expecting = "expected a list of globs, a map of projects, or both"
))]
pub enum WorkspaceProjects {
    /// Using both globs and explicit source paths.
    #[setting(nested)]
    Both(WorkspaceProjectsConfig),

    /// Using globs. Suffix with `moon.yml` or `moon.pkl` to be distinct.
    Globs(Vec<String>),

    /// Using a mapping of IDs to source paths.
    #[setting(default)]
    Sources(FxHashMap<Id, String>),
}

/// Configures all aspects of the moon workspace.
/// Docs: https://moonrepo.dev/docs/config/workspace
#[derive(Clone, Config, Debug)]
pub struct WorkspaceConfig {
    #[setting(
        default = "https://moonrepo.dev/schemas/workspace.json",
        rename = "$schema"
    )]
    pub schema: String,

    /// Configures code ownership rules for generating a `CODEOWNERS` file.
    #[setting(nested)]
    pub codeowners: CodeownersConfig,

    /// Configures boundaries and constraints between projects.
    #[setting(nested)]
    pub constraints: ConstraintsConfig,

    /// Configures Docker integration for the workspace.
    #[setting(nested)]
    pub docker: DockerConfig,

    /// Configures experiments across the entire moon workspace.
    #[setting(nested)]
    pub experiments: ExperimentsConfig,

    /// Extends another workspace configuration file. Supports a relative
    /// file path or a secure URL.
    #[setting(extend, validate = validate::extends_string)]
    pub extends: Option<String>,

    /// Configures extensions that can be executed with `moon ext`.
    #[setting(nested)]
    pub extensions: FxHashMap<Id, ExtensionConfig>,

    /// Configures the generator for scaffolding from templates.
    #[setting(nested)]
    pub generator: GeneratorConfig,

    /// Configures aspects of the content hashing engine.
    #[setting(nested)]
    pub hasher: HasherConfig,

    /// Configures how and where notifications are sent.
    #[setting(nested)]
    pub notifier: NotifierConfig,

    /// Configures all projects within the workspace to create a project graph.
    /// Accepts a list of globs, a mapping of projects to relative file paths,
    /// or both values.
    #[setting(nested, validate = validate_projects)]
    pub projects: WorkspaceProjects,

    /// Configures aspects of the task runner (also known as the action pipeline).
    #[setting(nested)]
    pub runner: RunnerConfig,

    /// Collects anonymous usage information, and checks for new moon versions.
    #[setting(default = true)]
    pub telemetry: bool,

    /// Configures the version control system (VCS).
    #[setting(nested)]
    pub vcs: VcsConfig,

    /// Requires a specific version of the `moon` binary.
    pub version_constraint: Option<VersionReq>,
}

impl WorkspaceConfig {
    pub fn inherit_default_plugins(&mut self) {
        for (id, extension) in default_extensions() {
            self.extensions.entry(id).or_insert(extension);
        }
    }
}

#[cfg(feature = "loader")]
impl WorkspaceConfig {
    pub fn load<R: AsRef<Path>, P: AsRef<Path>>(
        workspace_root: R,
        path: P,
    ) -> miette::Result<WorkspaceConfig> {
        use crate::config_cache::ConfigCache;
        use crate::validate::check_yml_extension;
        use moon_common::color;
        use schematic::ConfigLoader;

        let root = workspace_root.as_ref();

        let mut result = ConfigLoader::<WorkspaceConfig>::new()
            .set_cacher(ConfigCache::new(root))
            .set_help(color::muted_light(
                "https://moonrepo.dev/docs/config/workspace",
            ))
            .set_root(root)
            .file(check_yml_extension(path.as_ref()))?
            .load()?;

        result.config.inherit_default_plugins();

        Ok(result.config)
    }

    pub fn load_from<P: AsRef<Path>>(workspace_root: P) -> miette::Result<WorkspaceConfig> {
        use moon_common::consts;

        let workspace_root = workspace_root.as_ref();

        Self::load(
            workspace_root,
            workspace_root
                .join(consts::CONFIG_DIRNAME)
                .join(consts::CONFIG_WORKSPACE_FILENAME),
        )
    }
}
