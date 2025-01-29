use moon_common::consts::CONFIG_DIRNAME;
use schematic::ConfigError;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Default)]
pub struct ConfigFinder {
    _ignored: bool,
}

impl ConfigFinder {
    pub fn get_project_files(&self, project_root: &Path) -> Vec<PathBuf> {
        self.get_project_file_names()
            .into_iter()
            .map(|name| project_root.join(name))
            .collect()
    }

    pub fn get_project_file_names(&self) -> Vec<String> {
        self.get_file_names("moon")
    }

    pub fn get_scoped_tasks_files(&self, moon_dir: &Path) -> miette::Result<Vec<PathBuf>> {
        self.get_from_dir(moon_dir.join("tasks"))
    }

    pub fn get_tasks_files(&self, moon_dir: &Path) -> Vec<PathBuf> {
        self.get_tasks_file_names()
            .into_iter()
            .map(|name| moon_dir.join(name))
            .collect()
    }

    pub fn get_tasks_file_names(&self) -> Vec<String> {
        self.get_file_names("tasks")
    }

    pub fn get_template_files(&self, template_root: &Path) -> Vec<PathBuf> {
        self.get_template_file_names()
            .into_iter()
            .map(|name| template_root.join(name))
            .collect()
    }

    pub fn get_template_file_names(&self) -> Vec<String> {
        self.get_file_names("template")
    }

    pub fn get_toolchain_files(&self, workspace_root: &Path) -> Vec<PathBuf> {
        self.get_toolchain_file_names()
            .into_iter()
            .map(|name| workspace_root.join(CONFIG_DIRNAME).join(name))
            .collect()
    }

    pub fn get_toolchain_file_names(&self) -> Vec<String> {
        self.get_file_names("toolchain")
    }

    pub fn get_workspace_files(&self, workspace_root: &Path) -> Vec<PathBuf> {
        self.get_workspace_file_names()
            .into_iter()
            .map(|name| workspace_root.join(CONFIG_DIRNAME).join(name))
            .collect()
    }

    pub fn get_workspace_file_names(&self) -> Vec<String> {
        self.get_file_names("workspace")
    }

    pub fn get_debug_label(&self, name: &str, top_level: bool) -> String {
        let mut label = String::new();

        if top_level {
            label.push_str(CONFIG_DIRNAME);
            label.push('/');
        }

        label.push_str(name);
        label.push_str(".{pkl,yml}");

        label
    }

    pub fn get_file_names(&self, name: &str) -> Vec<String> {
        vec![format!("{name}.yml"), format!("{name}.pkl")]
    }

    #[allow(clippy::only_used_in_recursion)]
    pub fn get_from_dir(&self, dir: PathBuf) -> miette::Result<Vec<PathBuf>> {
        let mut files = vec![];

        if !dir.exists() {
            return Ok(files);
        }

        for entry in fs::read_dir(&dir)
            .map_err(|error| ConfigError::ReadFileFailed {
                path: dir,
                error: Box::new(error),
            })?
            .flatten()
        {
            let path = entry.path();
            let file_type = entry
                .file_type()
                .map_err(|error| ConfigError::ReadFileFailed {
                    path: path.to_path_buf(),
                    error: Box::new(error),
                })?;

            if file_type.is_file() {
                // Non-config files may be located in these folders,
                // so avoid failing when trying to parse it as a config
                if path
                    .extension()
                    .is_some_and(|ext| ext == "yml" || ext == "yaml" || ext == "pkl")
                {
                    files.push(path);
                }
            } else if file_type.is_dir() {
                files.extend(self.get_from_dir(path)?);
            }
        }

        Ok(files)
    }
}
