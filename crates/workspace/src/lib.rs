mod errors;

use errors::WorkspaceError;
use monolith_config::{constants, GlobalProjectConfig, WorkspaceConfig};
use monolith_project::ProjectGraph;
use monolith_toolchain::Toolchain;
use std::env;
use std::path::{Path, PathBuf};

/// Recursively attempt to find the workspace root by locating the ".monolith"
/// configuration folder, starting from the current working directory.
fn find_workspace_root(current_dir: PathBuf) -> Option<PathBuf> {
    let config_dir = current_dir.join(constants::CONFIG_DIRNAME);

    if config_dir.exists() {
        return Some(current_dir);
    }

    let parent_dir = current_dir.parent();

    match parent_dir {
        Some(dir) => find_workspace_root(dir.to_path_buf()),
        None => None,
    }
}

fn find_package_json(root_dir: &Path) -> Result<PathBuf, WorkspaceError> {
    let package_json_path = root_dir.join("package.json");

    if !package_json_path.exists() {
        return Err(WorkspaceError::MissingPackageJson);
    }

    Ok(package_json_path)
}

// project.yml
fn load_global_project_config(root_dir: &Path) -> Result<GlobalProjectConfig, WorkspaceError> {
    let config_path = root_dir
        .join(constants::CONFIG_DIRNAME)
        .join(constants::CONFIG_PROJECT_FILENAME);

    if !config_path.exists() {
        return Err(WorkspaceError::MissingGlobalProjectConfigFile);
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

    if !config_path.exists() {
        return Err(WorkspaceError::MissingWorkspaceConfigFile);
    }

    match WorkspaceConfig::load(config_path) {
        Ok(cfg) => Ok(cfg),
        Err(errors) => Err(WorkspaceError::InvalidWorkspaceConfigFile(errors)),
    }
}

#[derive(Debug)]
pub struct Workspace {
    /// Workspace configuration loaded from ".monolith/workspace.yml".
    pub config: WorkspaceConfig,

    /// The root of the workspace that contains the ".monolith" config folder.
    pub dir: PathBuf,

    /// Path to the root `package.json` file.
    pub package_json_path: PathBuf,

    /// The project graph, where each project is lazy loaded in.
    pub projects: ProjectGraph,

    /// The toolchain instance that houses all runtime tools/languages.
    pub toolchain: Toolchain,

    /// The current working directory.
    pub working_dir: PathBuf,
}

impl Workspace {
    /// Create a new workspace instance starting from the current working directory.
    /// Will locate the workspace root and load available configuration files.
    pub fn load() -> Result<Workspace, WorkspaceError> {
        let working_dir = env::current_dir().unwrap();

        // Find root dir
        let root_dir = match find_workspace_root(working_dir.clone()) {
            Some(dir) => dir.canonicalize().unwrap(),
            None => return Err(WorkspaceError::MissingConfigDir),
        };
        let package_json_path = find_package_json(&root_dir)?;

        // Load configs
        let config = load_workspace_config(&root_dir)?;
        let project_config = load_global_project_config(&root_dir)?;

        // Setup components
        let toolchain = Toolchain::new(&config, &root_dir)?;
        let projects = ProjectGraph::new(&root_dir, project_config, &config.projects);

        Ok(Workspace {
            config,
            dir: root_dir,
            package_json_path,
            projects,
            toolchain,
            working_dir,
        })
    }
}
