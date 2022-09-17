use crate::filters;
use crate::GeneratorError;
use lazy_static::lazy_static;
use moon_config::{
    format_error_line, format_figment_errors, ConfigError, TemplateConfig,
    TemplateFrontmatterConfig,
};
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
    Create,
    Replace,
    Skip,
}

#[derive(Debug, Eq, PartialEq)]
pub struct TemplateFile {
    /// Frontmatter extracted into a config.
    pub config: Option<TemplateFrontmatterConfig>,

    /// Rendered and frontmatter-free file content.
    pub content: String,

    /// Absolute path to destination.
    pub dest_path: PathBuf,

    /// Relative path from templates dir. Also acts as the engine name.
    pub name: String,

    /// Absolute path to source (in templates dir).
    pub source_path: PathBuf,

    /// File state and operation to commit.
    pub state: FileState,
}

impl TemplateFile {
    pub fn load(name: String, source_path: PathBuf) -> Self {
        TemplateFile {
            config: None,
            content: String::new(),
            dest_path: PathBuf::new(),
            name,
            source_path,
            state: FileState::Create,
        }
    }

    pub fn is_forced(&self) -> bool {
        match &self.config {
            Some(cfg) => cfg.force.unwrap_or_default(),
            None => false,
        }
    }

    pub fn is_skipped(&self) -> bool {
        match &self.config {
            Some(cfg) => cfg.skip.unwrap_or_default(),
            None => false,
        }
    }

    pub fn set_content(&mut self, content: String, dest: &Path) -> Result<(), ConfigError> {
        if content.starts_with("---") {
            if let Some(fm_end) = &content[3..].find("---") {
                let config = TemplateFrontmatterConfig::parse(&content[3..(fm_end - 1)])?;

                if let Some(to) = &config.to {
                    self.dest_path = dest.join(to);
                }

                self.config = Some(config);
                self.content = content[(fm_end + 3)..].to_owned();

                return Ok(());
            }
        }

        self.content = content;
        self.dest_path = dest.join(&self.name);

        Ok(())
    }

    pub fn should_write(&self) -> bool {
        matches!(self.state, FileState::Create) || matches!(self.state, FileState::Replace)
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

            files.push(TemplateFile::load(name, source_path));
        }

        // Do a second pass and render the content
        for file in &mut files {
            file.set_content(self.engine.render(&file.name, context)?, &dest)?;
        }

        // Sort so files are deterministic
        files.sort_by(|a, d| a.name.cmp(&d.name));

        self.files = files;

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

    /// Write the template file to the defined destination path.
    pub async fn write_file(&self, file: &TemplateFile) -> Result<(), GeneratorError> {
        match file.state {
            FileState::Replace => {
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
        fs::write(&file.dest_path, &file.content).await?;

        Ok(())
    }
}
