use crate::constants::ROOT_NODE_ID;
use crate::errors::ProjectError;
use monolith_config::constants::CONFIG_PROJECT_FILENAME;
use monolith_config::project::{FileGroups, ProjectID};
use monolith_config::{GlobalProjectConfig, PackageJson, PackageJsonValue, ProjectConfig};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub type ProjectsMap = HashMap<ProjectID, Project>;

// project.yml
fn load_project_config(
    root_dir: &Path,
    project_path: &str,
) -> Result<Option<ProjectConfig>, ProjectError> {
    let config_path = root_dir.join(&project_path).join(CONFIG_PROJECT_FILENAME);

    if config_path.exists() {
        return match ProjectConfig::load(&config_path) {
            Ok(cfg) => Ok(Some(cfg)),
            Err(error) => Err(ProjectError::InvalidConfigFile(
                String::from(project_path),
                error,
            )),
        };
    }

    Ok(None)
}

// package.json
fn load_package_json(
    root_dir: &Path,
    project_path: &str,
) -> Result<Option<PackageJsonValue>, ProjectError> {
    let package_path = root_dir.join(&project_path).join("package.json");

    if package_path.exists() {
        return match PackageJson::load(&package_path) {
            Ok(json) => Ok(Some(json)),
            Err(error) => Err(ProjectError::InvalidPackageJson(
                String::from(project_path),
                error.to_string(),
            )),
        };
    }

    Ok(None)
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Project {
    /// Project configuration loaded from "project.yml", if it exists.
    pub config: Option<ProjectConfig>,

    /// Absolute path to the project's root folder.
    pub dir: PathBuf,

    /// File groups specific to the project. Inherits all file groups from the global config.
    #[serde(rename = "fileGroups")]
    pub file_groups: FileGroups,

    /// Unique ID for the project. Is the LHS of the `projects` setting.
    pub id: ProjectID,

    /// Relative path of the project from the workspace root. Is the RHS of the `projects` setting.
    pub location: String,

    /// Loaded "package.json", if it exists.
    #[serde(skip)]
    pub package_json: Option<PackageJsonValue>,
}

impl Project {
    pub fn new(
        id: &str,
        location: &str,
        root_dir: &Path,
        global_config: &GlobalProjectConfig,
    ) -> Result<Project, ProjectError> {
        let dir = root_dir.join(&location);

        if !dir.exists() {
            return Err(ProjectError::MissingFilePath(String::from(location)));
        }

        let config = load_project_config(root_dir, location)?;
        let package_json = load_package_json(root_dir, location)?;
        let mut file_groups = global_config.file_groups.clone();

        // Override global configs with local
        if let Some(borrowed_config) = &config {
            if let Some(local_file_groups) = &borrowed_config.file_groups {
                file_groups.extend(local_file_groups.clone());
            }
        }

        Ok(Project {
            config,
            dir: dir.canonicalize().unwrap(),
            file_groups,
            id: String::from(id),
            location: String::from(location),
            package_json,
        })
    }

    /// Return a list of projects this project depends on.
    /// Will always depend on the special root project.
    pub fn get_dependencies(&self) -> Vec<String> {
        let mut depends_on = vec![ROOT_NODE_ID.to_owned()];

        if let Some(config) = &self.config {
            if let Some(config_depends) = &config.depends_on {
                depends_on.extend_from_slice(config_depends);
            }
        }

        depends_on
    }

    /// Return the project as a JSON string.
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap()
    }
}
