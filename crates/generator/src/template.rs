use crate::GeneratorError;
use moon_config::{format_error_line, format_figment_errors, ConfigError, TemplateConfig};
use moon_constants::CONFIG_TEMPLATE_FILENAME;
use moon_utils::{fs, path};
use std::path::{Path, PathBuf};
use tera::{Context, Tera};

#[derive(Debug, Eq, PartialEq)]
pub enum FileState {
    Created,
    Replaced,
    Skipped,
}

#[derive(Debug, Eq, PartialEq)]
pub struct TemplateFile {
    /// Absolute path to destination.
    pub dest_path: PathBuf,

    /// Did the file already exist at the destination.
    pub existed: bool,

    /// Relative path from templates dir. Also acts as the engine name.
    pub name: String,

    /// Should we overwrite an existing file.
    pub overwrite: bool,

    /// Absolute path to source (in templates dir).
    pub source_path: PathBuf,
}

impl TemplateFile {
    pub fn should_write(&self) -> bool {
        if self.existed && !self.overwrite {
            return false;
        }

        true
    }

    pub fn state(&self) -> FileState {
        match (self.existed, self.overwrite) {
            (true, true) => FileState::Replaced,
            (true, false) => FileState::Skipped,
            _ => FileState::Created,
        }
    }
}

#[derive(Debug)]
pub struct Template {
    pub config: TemplateConfig,
    pub engine: Tera,
    pub files: Vec<TemplateFile>,
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

        Ok(Template {
            config,
            engine: Tera::default(),
            files: vec![],
            name,
            root,
        })
    }

    /// Load all template files from the source directory and return a list
    /// of template file structs. These will later be used for rendering and generating.
    pub async fn load_files(&mut self, dest: &Path) -> Result<(), GeneratorError> {
        let mut files = vec![];

        for entry in fs::read_dir_all(&self.root).await? {
            // This is moons schema, so skip it
            if entry.file_name() == CONFIG_TEMPLATE_FILENAME {
                continue;
            }

            let source_path = entry.path();
            let name = path::to_virtual_string(source_path.strip_prefix(&self.root).unwrap())?;
            let dest_path = dest.join(&name);
            let existed = dest_path.exists();

            self.engine.add_template_file(&source_path, Some(&name))?;

            files.push(TemplateFile {
                dest_path,
                existed,
                name,
                overwrite: false,
                source_path,
            })
        }

        // Sort so files are deterministic
        files.sort_by(|a, d| a.name.cmp(&d.name));

        self.files = files;

        Ok(())
    }

    pub async fn render_file(
        &self,
        file: &TemplateFile,
        context: &Context,
    ) -> Result<(), GeneratorError> {
        let content = self.engine.render(&file.name, context)?;

        fs::create_dir_all(file.dest_path.parent().unwrap()).await?;

        fs::write(&file.dest_path, &content).await?;

        Ok(())
    }
}
