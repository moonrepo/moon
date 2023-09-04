use crate::template_file::{FileState, TemplateFile};
use crate::{filters, CodegenError};
use moon_common::consts::CONFIG_TEMPLATE_FILENAME;
use moon_common::path::{to_virtual_string, RelativePathBuf};
use moon_common::Id;
use moon_config::TemplateConfig;
use once_cell::sync::Lazy;
use regex::Regex;
use rustc_hash::FxHashMap;
use starbase_utils::{fs, json, yaml};
use std::path::{Path, PathBuf};
use tera::{Context, Tera};
use tracing::debug;

static PATH_VAR: Lazy<Regex> = Lazy::new(|| Regex::new(r"\[([A-Za-z0-9_]+)\]").unwrap());

#[derive(Debug)]
pub struct Template {
    pub assets: FxHashMap<PathBuf, RelativePathBuf>,
    pub config: TemplateConfig,
    pub engine: Tera,
    pub files: Vec<TemplateFile>,
    pub id: Id,
    pub root: PathBuf,
}

impl Template {
    pub fn new(id: Id, root: PathBuf) -> miette::Result<Template> {
        debug!(template = id.as_str(), root = ?root, "Loading template");

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
            assets: FxHashMap::default(),
            config: TemplateConfig::load_from(&root)?,
            engine,
            files: vec![],
            id,
            root,
        })
    }

    /// Load all template files from the source directory and return a list
    /// of template file structs. These will later be used for rendering and generating.
    pub fn load_files(&mut self, dest: &Path, context: &Context) -> miette::Result<()> {
        let mut files = vec![];

        debug!(
            template = self.id.as_str(),
            root = ?self.root,
            "Loading template files"
        );

        for entry in fs::read_dir_all(&self.root)? {
            // This is our schema, so skip it
            if entry.file_name() == CONFIG_TEMPLATE_FILENAME {
                continue;
            }

            let source_path = entry.path();
            let source_content = fs::read_file_bytes(&source_path)?;

            let name =
                self.interpolate_path(source_path.strip_prefix(&self.root).unwrap(), context)?;

            // Images, etc
            if content_inspector::inspect(&source_content).is_binary() {
                debug!(
                    template = self.id.as_str(),
                    file = name.as_str(),
                    source = ?source_path,
                    "Loading asset file",
                );

                self.assets.insert(source_path, name);
                continue;
            }

            let content = unsafe { String::from_utf8_unchecked(source_content) };

            self.engine
                .add_raw_template(name.as_str(), &content)
                .map_err(|error| CodegenError::LoadTemplateFileFailed {
                    path: source_path.clone(),
                    error,
                })?;

            // Add partials to Tera, but skip copying them
            if name.as_str().contains("partial") {
                debug!(
                    template = self.id.as_str(),
                    file = name.as_str(),
                    source = ?source_path,
                    "Skipping partial as a template file",
                );

                continue;
            }

            debug!(
                template = self.id.as_str(),
                file = name.as_str(),
                source = ?source_path,
                "Loading template file",
            );

            let mut file = TemplateFile::new(name, source_path);

            if file.raw {
                file.content = content;
            }

            files.push(file);
        }

        // Do a second pass and render the content
        for file in &mut files {
            if file.raw {
                continue;
            }

            file.set_content(
                self.engine
                    .render(file.name.as_str(), context)
                    .map_err(|error| CodegenError::RenderTemplateFileFailed {
                        path: file.source_path.clone(),
                        error,
                    })?,
                dest,
            )?;
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
    ) -> miette::Result<RelativePathBuf> {
        let mut name = to_virtual_string(path)?;

        // Remove template file extensions
        if let Some(name_prefix) = name.strip_suffix(".tera") {
            name = name_prefix.to_owned();
        }

        if let Some(name_prefix) = name.strip_suffix(".twig") {
            name = name_prefix.to_owned();
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
        let path = Tera::default()
            .render_str(&name, context)
            .map_err(|error| CodegenError::InterpolateTemplateFileFailed {
                path: name.to_owned(),
                error,
            })?;

        Ok(RelativePathBuf::from(path))
    }

    /// Write the template file to the defined destination path.
    pub fn write_file(&self, file: &TemplateFile) -> miette::Result<()> {
        match file.state {
            FileState::Merge => {
                debug!(
                    file = file.name.as_str(),
                    to = ?file.dest_path,
                    "Merging template file with destination",
                );

                match file.is_mergeable() {
                    Some("json") => {
                        let prev: json::JsonValue = json::read_file(&file.dest_path)?;
                        let next: json::JsonValue = json::read_file(&file.source_path)?;

                        json::write_file_with_config(
                            &file.dest_path,
                            json::merge(&prev, &next),
                            true,
                        )?;
                    }
                    Some("yaml") => {
                        let prev: yaml::YamlValue = yaml::read_file(&file.dest_path)?;
                        let next: yaml::YamlValue = yaml::read_file(&file.source_path)?;

                        yaml::write_file_with_config(&file.dest_path, &yaml::merge(&prev, &next))?;
                    }
                    _ => {}
                };
            }
            FileState::Replace => {
                debug!(
                    file = file.name.as_str(),
                    to = ?file.dest_path,
                    "Overwriting with template file",
                );

                fs::write_file(&file.dest_path, &file.content)?;
            }
            _ => {
                debug!(
                    file = file.name.as_str(),
                    to = ?file.dest_path,
                    "Writing template file",
                );

                fs::write_file(&file.dest_path, &file.content)?;
            }
        };

        Ok(())
    }
}
