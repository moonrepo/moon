use crate::filters;
use crate::GeneratorError;
use moon_config::{
    format_error_line, format_figment_errors, ConfigError, TemplateConfig,
    TemplateFrontmatterConfig,
};
use moon_constants::CONFIG_TEMPLATE_FILENAME;
use moon_logger::{color, debug, trace};
use moon_utils::{fs, json, lazy_static, path, regex, yaml};
use std::path::{Path, PathBuf};
use tera::{Context, Tera};

lazy_static! {
    pub static ref PATH_VAR: regex::Regex = regex::create_regex(r#"\[([A-Za-z0-9_]+)\]"#).unwrap();
}

const LOG_TARGET: &str = "moon:generator:template";

#[derive(Debug, Eq, PartialEq)]
pub enum FileState {
    Create,
    Merge,
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

    /// Relative path from templates dir. Also acts as the Tera engine name.
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

    pub fn is_mergeable<'l>(&self) -> Option<&'l str> {
        let mut ext = &self.name;

        if let Some(cfg) = &self.config {
            if let Some(to) = &cfg.to {
                ext = to;
            }
        }

        if ext.ends_with("json") {
            return Some("json");
        } else if ext.ends_with("yaml") || ext.ends_with("yml") {
            return Some("yaml");
        }

        None
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

    pub fn set_content<T: AsRef<str>>(
        &mut self,
        content: T,
        dest: &Path,
    ) -> Result<(), ConfigError> {
        let content = content.as_ref().trim_start();

        self.dest_path = dest.join(&self.name);

        if content.starts_with("---") {
            trace!(
                target: LOG_TARGET,
                "Found frontmatter in template file {}, extracting",
                color::file(&self.name),
            );

            if let Some(fm_end) = &content[4..].find("---") {
                let end_index = fm_end + 4;
                let config = TemplateFrontmatterConfig::parse(&content[4..end_index])?;

                if let Some(to) = &config.to {
                    self.dest_path = dest.join(to);
                }

                self.config = Some(config);
                self.content = content[(end_index + 4)..].trim_start().to_owned();

                return Ok(());
            }
        }

        self.content = content.to_owned();

        Ok(())
    }

    pub fn should_write(&self) -> bool {
        !matches!(self.state, FileState::Skip)
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
        engine.register_filter("lower_case", filters::lower_case);
        engine.register_filter("pascal_case", filters::pascal_case);
        engine.register_filter("snake_case", filters::snake_case);
        engine.register_filter("upper_case", filters::upper_case);
        engine.register_filter("upper_kebab_case", filters::upper_kebab_case);
        engine.register_filter("upper_snake_case", filters::upper_snake_case);
        engine.register_filter("path_join", filters::path_join);
        engine.register_filter("path_relative", filters::path_relative);

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
    pub fn load_files(&mut self, dest: &Path, context: &Context) -> Result<(), GeneratorError> {
        let mut files = vec![];

        for entry in fs::read_dir_all(&self.root)? {
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
            // if let Err(e) = self.engine.render(&file.name, context) {
            //     dbg!(e);
            // }

            file.set_content(self.engine.render(&file.name, context)?, dest)?;
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
        let mut name = path::to_virtual_string(path)?;

        // Remove template file extensions
        if name.ends_with(".tera") {
            name = name.strip_suffix(".tera").unwrap().to_owned();
        }

        if name.ends_with(".twig") {
            name = name.strip_suffix(".twig").unwrap().to_owned();
        }

        // Replace [var] with {{ var }} syntax
        name = PATH_VAR
            .replace_all(&name, |caps: &regex::Captures| {
                if let Some(var) = caps.get(1) {
                    let var = var.as_str();

                    if context.contains_key(var) {
                        return format!("{{{{ {var} | as_str }}}}");
                    }
                }

                caps.get(0).unwrap().as_str().to_owned()
            })
            .to_string();

        // Render the path to interpolate the values
        Ok(Tera::default().render_str(&name, context)?)
    }

    /// Write the template file to the defined destination path.
    pub fn write_file(&self, file: &TemplateFile) -> Result<(), GeneratorError> {
        match file.state {
            FileState::Merge => {
                trace!(
                    target: LOG_TARGET,
                    "Merging template file {} with {}",
                    color::file(&file.name),
                    color::path(&file.dest_path)
                );
            }
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

        fs::create_dir_all(file.dest_path.parent().unwrap())?;

        if matches!(file.state, FileState::Merge) {
            match file.is_mergeable() {
                Some("json") => {
                    let prev: json::JsonValue = json::read(&file.dest_path)?;
                    let next: json::JsonValue = json::read(&file.source_path)?;

                    json::write(&file.dest_path, &json::merge(&prev, &next), true)?;
                }
                Some("yaml") => {
                    let prev: yaml::YamlValue = yaml::read(&file.dest_path)?;
                    let next: yaml::YamlValue = yaml::read(&file.source_path)?;

                    yaml::write(&file.dest_path, &yaml::merge(&prev, &next))?;
                }
                _ => {}
            }
        } else {
            fs::write(&file.dest_path, &file.content)?;
        }

        Ok(())
    }
}
