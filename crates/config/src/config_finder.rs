use crate::validate::check_yml_extension;
use moon_common::consts::CONFIG_DIRNAME;
use moon_common::supports_pkl_configs;
use schematic::{Config, ConfigLoader};
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct ConfigFinder {
    pkl: bool,
}

impl Default for ConfigFinder {
    fn default() -> Self {
        Self {
            pkl: supports_pkl_configs(),
        }
    }
}

impl ConfigFinder {
    pub fn prepare_loader<T: Config>(
        &self,
        loader: &mut ConfigLoader<T>,
        files: Vec<PathBuf>,
    ) -> miette::Result<()> {
        for file in files {
            if file
                .extension()
                .is_some_and(|ext| ext == "yaml" || ext == "yml")
            {
                loader.file_optional(check_yml_extension(&file))?;
            } else {
                loader.file_optional(file)?;
            }
        }

        Ok(())
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

    fn get_file_names(&self, name: &str) -> Vec<String> {
        let mut files = vec![format!("{name}.yml")];

        if self.pkl {
            files.push(format!("{name}.pkl"));
        }

        files
    }
}
