use crate::errors::ProjectError;
use monolith_config::constants::CONFIG_PROJECT_FILENAME;
use monolith_config::project::{FileGroups, ProjectID};
use monolith_config::{GlobalProjectConfig, PackageJson, PackageJsonValue, ProjectConfig};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

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

#[derive(Debug, Deserialize, Serialize)]
pub struct Project {
    /// Unique identifier for the project. Is the LHS of the `projects` setting.
    pub id: ProjectID,

    /// Project configuration loaded from "project.yml", if it exists.
    pub config: Option<ProjectConfig>,

    /// Absolute path to the project's root folder.
    pub dir: PathBuf,

    /// File groups specific to the project. Inherits all file groups from the global config.
    #[serde(rename = "fileGroups")]
    pub file_groups: FileGroups,

    /// Relative path of the project from the workspace root. Is the RHS of the `projects` setting.
    pub location: String,

    /// Loaded "package.json", if it exists.
    #[serde(skip)]
    pub package_json: Option<PackageJsonValue>,
}

impl Project {
    pub fn new(
        project_id: &str,
        project_path: &str,
        root_dir: &Path,
        global_config: &GlobalProjectConfig,
    ) -> Result<Project, ProjectError> {
        let dir = root_dir.join(&project_path).canonicalize().unwrap();

        if !dir.exists() {
            return Err(ProjectError::DoesNotExist(String::from(project_path)));
        }

        let config = load_project_config(root_dir, project_path)?;
        let package_json = load_package_json(root_dir, project_path)?;
        let mut file_groups = global_config.file_groups.clone();

        // Override global configs with local
        if config.is_some() {
            let borrowed_config = config.as_ref().unwrap();

            if let Some(local_file_groups) = &borrowed_config.file_groups {
                file_groups.extend(local_file_groups.clone());
            }
        }

        Ok(Project {
            id: String::from(project_id),
            config,
            dir,
            file_groups,
            location: String::from(project_path),
            package_json,
        })
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap()
    }
}
