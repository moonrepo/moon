use crate::codegen_error::CodegenError;
use crate::template::Template;
use moon_common::consts::CONFIG_TEMPLATE_FILENAME;
use moon_common::path::RelativePathBuf;
use moon_common::Id;
use moon_config::{load_template_config_template, GeneratorConfig};
use starbase_utils::fs;
use std::path::{Path, PathBuf};
use tracing::debug;

pub struct CodeGenerator<'app> {
    config: &'app GeneratorConfig,
    workspace_root: &'app Path,
}

impl<'app> CodeGenerator<'app> {
    pub fn new(workspace_root: &'app Path, config: &'app GeneratorConfig) -> CodeGenerator<'app> {
        debug!("Creating code generator");

        CodeGenerator {
            config,
            workspace_root,
        }
    }

    /// Create a new template with a schema, using the first configured template path.
    /// Will error if a template of the same name already exists.
    pub fn create_template(&self, id: &str) -> miette::Result<Template> {
        let id = Id::clean(id)?;
        let root = self.create_absolute_path(self.config.templates[0].as_str(), id.as_str());

        if root.exists() {
            return Err(CodegenError::ExistingTemplate(id, root).into());
        }

        debug!(
            template = id.as_str(),
            to = ?root,
            "Creating new template",
        );

        fs::write_file(
            root.join(CONFIG_TEMPLATE_FILENAME),
            load_template_config_template(),
        )?;

        Template::new(id, root)
    }

    /// Load the template with the provided name, using the first match amongst
    /// the list of template paths. Will error if no match is found.
    pub fn load_template(&self, id: &str) -> miette::Result<Template> {
        let id = Id::clean(id)?;

        debug!(
            template = id.as_str(),
            locations = ?self.config.templates.iter().map(|t| t.as_str()).collect::<Vec<_>>(),
            "Attempting to find template from configured locations",
        );

        for template_path in &self.config.templates {
            let root = self.create_absolute_path(template_path.as_str(), id.as_str());

            if root.exists() {
                debug!(
                    template = id.as_str(),
                    root = ?root,
                    "Found template"
                );

                return Template::new(id, root);
            }
        }

        Err(CodegenError::MissingTemplate(id).into())
    }

    pub fn generate(&self, template: &Template) -> miette::Result<()> {
        debug!(template = template.id.as_str(), "Generating template files");

        for file in &template.files {
            if file.should_write() {
                template.write_file(file)?;
            }
        }

        debug!(template = template.id.as_str(), "Code generation complete!");

        Ok(())
    }

    fn create_absolute_path(&self, template_path: &str, template_name: &str) -> PathBuf {
        RelativePathBuf::from(template_path)
            .join(template_name)
            .normalize()
            .to_logical_path(self.workspace_root)
    }
}
