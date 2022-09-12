use crate::filters;
use crate::GeneratorError;
use lazy_static::lazy_static;
use moon_config::{format_error_line, format_figment_errors, ConfigError, TemplateConfig};
use moon_constants::CONFIG_TEMPLATE_FILENAME;
use moon_logger::{color, debug, trace};
use moon_utils::{fs, path, regex};
use std::path::{Path, PathBuf};
use tera::{Context, Tera};

lazy_static! {
    pub static ref PATH_VAR: regex::Regex = regex::create_regex(r#"\[([A-Za-z0-9_]+)\]"#).unwrap();
}

const LOG_TARGET: &str = "moon:generator:template";

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
        debug!(
            target: LOG_TARGET,
            "Loading template {} from {}",
            color::id(&name),
            color::path(&root)
        );

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

        let mut engine = Tera::default();
        engine.register_filter("camel_case", filters::camel_case);
        engine.register_filter("kebab_case", filters::kebab_case);
        engine.register_filter("pascal_case", filters::pascal_case);
        engine.register_filter("snake_case", filters::snake_case);
        engine.register_filter("upper_kebab_case", filters::upper_kebab_case);
        engine.register_filter("upper_snake_case", filters::upper_snake_case);

        Ok(Template {
            config,
            engine,
            files: vec![],
            name,
            root,
        })
    }

    /// Load all template files from the source directory and return a list
    /// of template file structs. These will later be used for rendering and generating.
    pub async fn load_files(
        &mut self,
        dest: &Path,
        context: &Context,
    ) -> Result<(), GeneratorError> {
        let mut files = vec![];

        for entry in fs::read_dir_all(&self.root).await? {
            // This is moon's schema, so skip it
            if entry.file_name() == CONFIG_TEMPLATE_FILENAME {
                continue;
            }

            let source_path = entry.path();
            let name =
                self.interpolate_path(source_path.strip_prefix(&self.root).unwrap(), context)?;
            let dest_path = dest.join(&name);
            let existed = dest_path.exists();

            self.engine.add_template_file(&source_path, Some(&name))?;

            // Add partials to Tera, but skip copying them
            if name.contains("partial") {
                continue;
            }

            trace!(
                target: LOG_TARGET,
                "Loading template file {} (source = {})",
                color::file(&name),
                color::path(&source_path),
            );

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

    /// Render the template file with the provided context, and write it to the file
    /// system at the defined destination path.
    pub async fn render_file(
        &self,
        file: &TemplateFile,
        context: &Context,
    ) -> Result<(), GeneratorError> {
        match file.state() {
            FileState::Replaced => {
                trace!(
                    target: LOG_TARGET,
                    "Overwriting template file {} (destination = {})",
                    color::file(&file.name),
                    color::path(&file.dest_path)
                );
            }
            _ => {
                trace!(
                    target: LOG_TARGET,
                    "Writing template file {} (destination = {})",
                    color::file(&file.name),
                    color::path(&file.dest_path)
                );
            }
        }

        fs::create_dir_all(file.dest_path.parent().unwrap()).await?;

        fs::write(
            &file.dest_path,
            // Render the template and interpolate the values
            self.engine.render(&file.name, context)?,
        )
        .await?;

        Ok(())
    }

    /// Tera *does not* support iterating over the context, so we're unable
    /// to interpolate a path ourselves. Instead, let's use Tera and its
    /// template rendering to handle this.
    pub fn interpolate_path(
        &self,
        path: &Path,
        context: &Context,
    ) -> Result<String, GeneratorError> {
        let name = path::to_virtual_string(path)?;

        // Replace [var] with {{ var }} syntax
        let name = PATH_VAR
            .replace_all(&name, |caps: &regex::Captures| {
                if let Some(var) = caps.get(1) {
                    let var = var.as_str();

                    if context.contains_key(var) {
                        return format!("{{{{ {} | as_str }}}}", var);
                    }
                }

                caps.get(0).unwrap().as_str().to_owned()
            })
            .to_string();

        // Render the path to interpolate the values
        Ok(Tera::default().render_str(&name, context)?)
    }
}
