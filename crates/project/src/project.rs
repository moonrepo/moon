use crate::errors::ProjectError;
use monolith_config::constants::CONFIG_PROJECT_FILENAME;
use monolith_config::project::{FileGroups, ProjectID};
use monolith_config::{GlobalProjectConfig, PackageJson, PackageJsonValue, ProjectConfig};
use std::cmp::Ordering;
use std::hash::{Hash, Hasher};
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

#[derive(Debug)]
pub struct Project {
    /// Unique identifier for the project. Is the LHS of the `projects` setting.
    pub id: ProjectID,

    /// Project configuration loaded from "project.yml", if it exists.
    pub config: Option<ProjectConfig>,

    /// Absolute path to the project's root folder.
    pub dir: PathBuf,

    /// File groups specific to the project. Inherits all file groups from the global config.
    pub file_groups: FileGroups,

    /// Relative path of the project from the workspace root. Is the RHS of the `projects` setting.
    pub location: String,

    /// Loaded "package.json", if it exists.
    pub package_json: Option<PackageJsonValue>,
}

impl Project {
    pub fn new(
        project_id: &str,
        project_path: &str,
        root_dir: &Path,
        global_config: &GlobalProjectConfig,
    ) -> Result<Project, ProjectError> {
        let config = load_project_config(root_dir, &project_path)?;
        let package_json = load_package_json(root_dir, &project_path)?;
        let mut file_groups = global_config.file_groups.0.clone();

        // Override global configs with local
        if config.is_some() {
            let borrowed_config = config.as_ref().unwrap();

            if let Some(local_file_groups) = &borrowed_config.file_groups {
                file_groups.extend(local_file_groups.0.clone());
            }
        }

        Ok(Project {
            id: String::from(project_id),
            config,
            dir: root_dir.join(&project_path),
            file_groups: FileGroups(file_groups),
            location: String::from(project_path),
            package_json,
        })
    }
}

// impl Clone for Project {
//     fn clone(&self) -> Project {
//         *self
//     }
// }

// impl Hash for Project {
//     fn hash<H: Hasher>(&self, state: &mut H) {
//         self.id.hash(state);
//     }
// }

// impl PartialEq for Project {
//     fn eq(&self, other: &Self) -> bool {
//         self.id == other.id
//     }
// }

// impl Eq for Project {}

// impl Ord for Project {
//     fn cmp(&self, other: &Self) -> Ordering {
//         self.id.cmp(&other.id)
//     }
// }

// impl PartialOrd for Project {
//     fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
//         Some(self.cmp(other))
//     }
// }
