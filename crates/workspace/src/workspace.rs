use crate::errors::WorkspaceError;
use crate::vcs::{Vcs, VcsManager};
use moon_cache::CacheEngine;
use moon_config::package::PackageJson;
use moon_config::tsconfig::TsConfigJson;
use moon_config::{constants, GlobalProjectConfig, WorkspaceConfig};
use moon_logger::{color, debug, trace};
use moon_project::ProjectGraph;
use moon_toolchain::Toolchain;
use std::env;
use std::path::{Path, PathBuf};

/// Recursively attempt to find the workspace root by locating the ".moon"
/// configuration folder, starting from the current working directory.
fn find_workspace_root(current_dir: PathBuf) -> Option<PathBuf> {
    let config_dir = current_dir.join(constants::CONFIG_DIRNAME);

    trace!(
        target: "moon:workspace",
        "Attempting to find workspace root at {}",
        color::path(&current_dir),
    );

    if config_dir.exists() {
        return Some(current_dir);
    }

    let parent_dir = current_dir.parent();

    match parent_dir {
        Some(dir) => find_workspace_root(dir.to_path_buf()),
        None => None,
    }
}

// project.yml
fn load_global_project_config(root_dir: &Path) -> Result<GlobalProjectConfig, WorkspaceError> {
    let config_path = root_dir
        .join(constants::CONFIG_DIRNAME)
        .join(constants::CONFIG_PROJECT_FILENAME);

    trace!(
        target: "moon:workspace",
        "Attempting to find {} in {}",
        color::file(
            &format!("{}/{}",
                constants::CONFIG_DIRNAME,
                constants::CONFIG_PROJECT_FILENAME,
            )
        ),
        color::path(root_dir)
    );

    if !config_path.exists() {
        return Ok(GlobalProjectConfig::default());
    }

    match GlobalProjectConfig::load(config_path) {
        Ok(cfg) => Ok(cfg),
        Err(errors) => Err(WorkspaceError::InvalidGlobalProjectConfigFile(errors)),
    }
}

// workspace.yml
fn load_workspace_config(root_dir: &Path) -> Result<WorkspaceConfig, WorkspaceError> {
    let config_path = root_dir
        .join(constants::CONFIG_DIRNAME)
        .join(constants::CONFIG_WORKSPACE_FILENAME);

    trace!(
        target: "moon:workspace",
        "Attempting to find {} in {}",
        color::file(
            &format!("{}/{}",
                constants::CONFIG_DIRNAME,
                constants::CONFIG_WORKSPACE_FILENAME,
            )
        ),
        color::path(root_dir)
    );

    if !config_path.exists() {
        return Err(WorkspaceError::MissingWorkspaceConfigFile);
    }

    match WorkspaceConfig::load(config_path) {
        Ok(cfg) => Ok(cfg),
        Err(errors) => Err(WorkspaceError::InvalidWorkspaceConfigFile(errors)),
    }
}

pub struct Workspace {
    /// Engine for reading and writing cache/outputs.
    pub cache: CacheEngine,

    /// Workspace configuration loaded from ".moon/workspace.yml".
    pub config: WorkspaceConfig,

    /// The project graph, where each project is lazy loaded in.
    pub projects: ProjectGraph,

    /// The root of the workspace that contains the ".moon" config folder.
    pub root: PathBuf,

    /// The toolchain instance that houses all runtime tools/languages.
    pub toolchain: Toolchain,

    /// The current working directory.
    pub working_dir: PathBuf,
}

impl Workspace {
    /// Create a new workspace instance starting from the current working directory.
    /// Will locate the workspace root and load available configuration files.
    pub async fn load() -> Result<Workspace, WorkspaceError> {
        let working_dir = env::current_dir().unwrap();
        let root_dir = match find_workspace_root(working_dir.clone()) {
            Some(dir) => dir,
            None => return Err(WorkspaceError::MissingConfigDir),
        };

        debug!(
            target: "moon:workspace",
            "Creating workspace at {} (from working directory {})",
            color::path(&root_dir),
            color::path(&working_dir)
        );

        // Load configs
        let config = load_workspace_config(&root_dir)?;
        let project_config = load_global_project_config(&root_dir)?;

        // Setup components
        let cache = CacheEngine::create(&root_dir).await?;
        let toolchain = Toolchain::create(&root_dir, &config).await?;
        let projects = ProjectGraph::new(&root_dir, project_config, &config.projects);

        Ok(Workspace {
            cache,
            config,
            projects,
            root: root_dir,
            toolchain,
            working_dir,
        })
    }

    /// Detect the version control system currently being used.
    pub fn detect_vcs(&self) -> Box<dyn Vcs + Send + Sync> {
        VcsManager::load(&self.config, &self.working_dir)
    }

    /// Load and parse the root `package.json`.
    pub async fn load_package_json(&self) -> Result<PackageJson, WorkspaceError> {
        let package_json_path = self.root.join("package.json");

        trace!(
            target: "moon:workspace",
            "Attempting to find {} in {}",
            color::file("package.json"),
            color::path(&self.root),
        );

        if !package_json_path.exists() {
            return Err(WorkspaceError::MissingPackageJson);
        }

        Ok(PackageJson::load(&package_json_path).await?)
    }

    /// Load and parse the root `tsconfig.json` if it exists.
    pub async fn load_tsconfig_json(
        &self,
        tsconfig_name: &str,
    ) -> Result<Option<TsConfigJson>, WorkspaceError> {
        let tsconfig_json_path = self.root.join(tsconfig_name);

        trace!(
            target: "moon:workspace",
            "Attempting to find {} in {}",
            color::file(tsconfig_name),
            color::path(&self.root),
        );

        if !tsconfig_json_path.exists() {
            return Ok(None);
        }

        Ok(Some(TsConfigJson::load(&tsconfig_json_path).await?))
    }
}
