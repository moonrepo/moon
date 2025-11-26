use schematic::ConfigError;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct ConfigFinder {
    extensions: Vec<String>,
}

impl Default for ConfigFinder {
    fn default() -> Self {
        Self {
            // In resolution order
            extensions: vec![
                "yml".into(),
                "yaml".into(),
                "jsonc".into(),
                "json".into(),
                "pkl".into(),
                "hcl".into(),
            ],
        }
    }
}

impl ConfigFinder {
    pub fn get_extensions_file_names(&self) -> Vec<String> {
        self.get_file_names("extensions")
    }

    pub fn get_project_file_names(&self) -> Vec<String> {
        self.get_file_names("moon")
    }

    pub fn get_template_file_names(&self) -> Vec<String> {
        self.get_file_names("template")
    }

    pub fn get_toolchains_file_names(&self) -> Vec<String> {
        self.get_file_names("toolchains")
    }

    pub fn get_workspace_file_names(&self) -> Vec<String> {
        self.get_file_names("workspace")
    }

    pub fn get_debug_label(&self, name: &str) -> String {
        format!("{name}.{}", self.get_ext_glob())
    }

    pub fn get_debug_label_root(&self, name: &str, dir: &Path) -> String {
        let mut label = String::new();
        let ext_glob = self.get_ext_glob();

        if dir.file_name().is_some_and(|inner| inner == ".moon") {
            label.push_str(".moon/");
        } else {
            label.push_str(".config/moon/");
        }

        label.push_str(name);
        label.push('.');
        label.push_str(&ext_glob);
        label
    }

    pub fn get_ext_glob(&self) -> String {
        format!("{{{}}}", self.extensions.join(","))
    }

    pub fn get_file_names(&self, name: &str) -> Vec<String> {
        self.extensions
            .iter()
            .map(|ext| format!("{name}.{ext}"))
            .collect()
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
                    .is_some_and(|ext| self.extensions.iter().any(|e| ext == e.as_str()))
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
