use crate::asset_file::AssetFile;
use crate::template_file::{FileState, MergeType, TemplateFile};
use crate::{filters, funcs, CodegenError};
use miette::IntoDiagnostic;
use moon_common::consts::{CONFIG_TEMPLATE_FILENAME_PKL, CONFIG_TEMPLATE_FILENAME_YML};
use moon_common::path::{to_virtual_string, RelativePathBuf};
use moon_common::Id;
use moon_config::TemplateConfig;
use once_cell::sync::Lazy;
use regex::Regex;
use starbase_utils::{fs, json, yaml};
use std::collections::BTreeMap;
use std::mem;
use std::path::{Path, PathBuf};
use tera::{Context, Tera};
use tracing::{debug, instrument};

static PATH_VAR: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\[([A-Za-z0-9_]+)(?:\s*\|\s*([^\]]+))?\]").unwrap());

#[derive(Clone, Debug)]
pub struct Template {
    pub assets: BTreeMap<RelativePathBuf, AssetFile>,
    pub config: TemplateConfig,
    pub engine: Tera,
    pub files: BTreeMap<RelativePathBuf, TemplateFile>,
    pub id: Id,
    pub root: PathBuf,
    pub templates: Vec<Template>, // Extending
}

impl Template {
    pub fn new(id: Id, root: PathBuf) -> miette::Result<Template> {
        debug!(root = ?root, "Loading template");

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
        engine.register_function("variables", funcs::variables);

        let config = TemplateConfig::load_from(&root)?;

        Ok(Template {
            id: config.id.clone().unwrap_or(id),
            assets: BTreeMap::new(),
            config,
            engine,
            files: BTreeMap::new(),
            root,
            templates: vec![],
        })
    }

    /// Extend another template and include its files when generating.
    /// Furthermore, we'll also merge variables so that they can be handled
    /// in the command correctly.
    #[instrument(skip_all)]
    pub fn extend_template(&mut self, mut template: Template) {
        for (key, config) in mem::take(&mut template.config.variables) {
            self.config.variables.entry(key).or_insert(config);
        }

        self.templates.push(template);
    }

    /// Once files have been loaded by all templates in the extends chain,
    /// we must flatten all nested files map into a single top-level map.
    #[instrument(skip_all)]
    pub fn load_extended_files(&mut self, dest: &Path, context: &Context) -> miette::Result<()> {
        if self.templates.is_empty() {
            return Ok(());
        }

        let mut assets = BTreeMap::new();
        let mut files = BTreeMap::new();

        for template in &mut self.templates {
            template.load_files(dest, context)?;

            self.engine.extend(&template.engine).into_diagnostic()?;

            assets.extend(mem::take(&mut template.assets));
            files.extend(mem::take(&mut template.files));
        }

        assets.extend(mem::take(&mut self.assets));
        files.extend(mem::take(&mut self.files));

        self.assets = assets;
        self.files = files;

        Ok(())
    }

    /// Load all template files from the source directory and return a list
    /// of template file structs. These will later be used for rendering and generating.
    #[instrument(skip_all)]
    pub fn load_files(&mut self, dest: &Path, context: &Context) -> miette::Result<()> {
        self.load_extended_files(dest, context)?;

        let mut files = vec![];

        debug!(
            template = self.id.as_str(),
            root = ?self.root,
            "Loading template files"
        );

        for entry in fs::read_dir_all(&self.root)? {
            // This is our schema, so skip it
            if entry.file_name() == CONFIG_TEMPLATE_FILENAME_YML
                || entry.file_name() == CONFIG_TEMPLATE_FILENAME_PKL
            {
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

                self.assets.insert(
                    name.clone(),
                    AssetFile {
                        content: source_content,
                        dest_path: name.to_logical_path(dest),
                        name,
                        source_path,
                    },
                );

                continue;
            }

            let content = unsafe { String::from_utf8_unchecked(source_content) };

            // Add partial templates to Tera, but skip including them as a file
            if name.as_str().contains("partial") {
                self.engine
                    .add_raw_template(name.as_str(), &content)
                    .map_err(|error| CodegenError::LoadTemplateFileFailed {
                        path: source_path.clone(),
                        error: Box::new(error),
                    })?;

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
            } else {
                self.engine
                    .add_raw_template(file.name.as_str(), &content)
                    .map_err(|error| CodegenError::LoadTemplateFileFailed {
                        path: file.source_path.clone(),
                        error: Box::new(error),
                    })?;
            }

            files.push(file);
        }

        // Do a second pass and render the content
        for mut file in files {
            if file.raw {
                file.set_raw_content(dest)?;
            } else {
                file.set_content(
                    self.engine
                        .render(file.name.as_str(), context)
                        .map_err(|error| CodegenError::RenderTemplateFileFailed {
                            path: file.source_path.clone(),
                            error: Box::new(error),
                        })?,
                    dest,
                )?;
            }

            self.files.insert(file.name.clone(), file);
        }

        Ok(())
    }

    /// Tera *does not* support iterating over the context, so we're unable
    /// to interpolate a path ourselves. Instead, let's use Tera and its
    /// template rendering to handle this.
    pub fn interpolate_path(
        &mut self,
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
                        return format!(
                            "{{{{ {var} | as_str {} }}}}",
                            caps.get(2)
                                .map(|f| format!("| {}", f.as_str()))
                                .unwrap_or_default()
                        );
                    }
                }

                caps.get(0).unwrap().as_str().to_owned()
            })
            .to_string();

        // Render the path to interpolate the values
        let path = self.engine.render_str(&name, context).map_err(|error| {
            CodegenError::InterpolateTemplateFileFailed {
                path: name.to_owned(),
                error: Box::new(error),
            }
        })?;

        Ok(RelativePathBuf::from(path))
    }

    /// Copy the asset file to the defined destination path.
    pub fn copy_asset(&self, file: &AssetFile) -> miette::Result<()> {
        debug!(
            file = file.name.as_str(),
            to = ?file.dest_path,
            "Copying asset file",
        );

        fs::write_file(&file.dest_path, &file.content)?;

        Ok(())
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
                    Some(MergeType::Json) => {
                        let prev: json::JsonValue = json::read_file(&file.dest_path)?;
                        let next: json::JsonValue = json::parse(&file.content)?;

                        json::write_file_with_config(
                            &file.dest_path,
                            &json::merge(&prev, &next),
                            true,
                        )?;
                    }
                    Some(MergeType::Yaml) => {
                        let prev: yaml::YamlValue = yaml::read_file(&file.dest_path)?;
                        let next: yaml::YamlValue = yaml::parse(&file.content)?;

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
