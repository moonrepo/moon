use crate::GeneratorError;
use moon_config::{format_error_line, format_figment_errors, ConfigError, TemplateConfig};
use moon_constants::CONFIG_TEMPLATE_FILENAME;
use moon_error::MoonError;
use moon_utils::fs;
use std::path::{Path, PathBuf};

pub enum FileState {
    Created,
    Replaced,
    Skipped,
}

#[derive(Debug)]
pub struct TemplateFile {
    /// Absolute path to destination.
    pub dest_path: PathBuf,

    /// Did the file already exist at the destination.
    pub existed: bool,

    /// Should we overwrite an existing file.
    pub overwrite: bool,

    /// Relative path from templates dir.
    pub path: PathBuf,

    /// Absolute path to source (in templates dir).
    pub source_path: PathBuf,
}

impl TemplateFile {
    pub async fn copy(&self) -> Result<bool, MoonError> {
        if self.existed && !self.overwrite {
            return Ok(false);
        }

        fs::create_dir_all(self.dest_path.parent().unwrap()).await?;
        fs::copy_file(&self.source_path, &self.dest_path).await?;

        Ok(true)
    }

    pub fn state(&self) -> FileState {
        match (self.existed, self.overwrite) {
            (true, true) => FileState::Replaced,
            (true, false) => FileState::Skipped,
            _ => FileState::Created,
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct Template {
    pub config: TemplateConfig,
    pub name: String,
    pub root: PathBuf,
}

impl Template {
    pub fn new(name: String, root: PathBuf) -> Result<Template, GeneratorError> {
        let config = match TemplateConfig::load(root.join(CONFIG_TEMPLATE_FILENAME)) {
            Ok(cfg) => cfg,
            Err(errors) => {
                return Err(GeneratorError::InvalidConfigFile(
                    if let ConfigError::FailedValidation(valids) = errors {
                        format_figment_errors(valids)
                    } else {
                        format_error_line(errors.to_string())
                    },
                ));
            }
        };

        Ok(Template { config, name, root })
    }

    pub async fn get_template_files(
        &self,
        dest: &Path,
    ) -> Result<Vec<TemplateFile>, GeneratorError> {
        let mut files = vec![];

        for entry in fs::read_dir_all(&self.root).await? {
            // This is moons schema, so skip it
            if entry.file_name() == CONFIG_TEMPLATE_FILENAME {
                continue;
            }

            let source_path = entry.path();
            let path = source_path.strip_prefix(&self.root).unwrap();
            let dest_path = dest.join(path);
            let existed = dest_path.exists();

            files.push(TemplateFile {
                dest_path,
                existed,
                overwrite: false,
                path: path.to_path_buf(),
                source_path,
            })
        }

        Ok(files)
    }
}
