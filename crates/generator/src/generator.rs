use crate::errors::GeneratorError;
use crate::template::Template;
use futures::future::try_join_all;
use moon_config::{load_template_config_template, GeneratorConfig};
use moon_constants::CONFIG_TEMPLATE_FILENAME;
use moon_logger::{color, debug, map_list, trace};
use moon_utils::{fs, path, regex::clean_id};
use std::path::{Path, PathBuf};

const LOG_TARGET: &str = "moon:generator";

pub struct Generator {
    config: GeneratorConfig,

    workspace_root: PathBuf,
}

impl Generator {
    pub fn create(workspace_root: &Path, config: &GeneratorConfig) -> Result<Self, GeneratorError> {
        debug!(target: LOG_TARGET, "Creating generator");

        Ok(Generator {
            config: config.to_owned(),
            workspace_root: workspace_root.to_path_buf(),
        })
    }

    /// Create a new template with a schema, using the first configured template path.
    /// Will error if a template of the same name already exists.
    pub async fn create_template(&self, name: &str) -> Result<Template, GeneratorError> {
        let name = clean_id(name);
        let root = self
            .workspace_root
            .join(&self.config.templates[0])
            .join(&name);

        if root.exists() {
            return Err(GeneratorError::ExistingTemplate(name, root));
        }

        debug!(
            target: LOG_TARGET,
            "Creating new template {} at {}",
            color::id(&name),
            color::path(&root)
        );

        fs::create_dir_all(&root).await?;

        fs::write(
            root.join(CONFIG_TEMPLATE_FILENAME),
            load_template_config_template(),
        )
        .await?;

        Template::new(name, root)
    }

    /// Load the template with the provided name, using the first match amongst
    /// the list of template paths. Will error if no match is found.
    pub async fn load_template(&self, name: &str) -> Result<Template, GeneratorError> {
        let name = clean_id(name);

        trace!(
            target: LOG_TARGET,
            "Finding template {} from configured locations: {}",
            color::id(&name),
            map_list(&self.config.templates, |t| color::file(t))
        );

        for template_path in &self.config.templates {
            let root = path::normalize(self.workspace_root.join(template_path).join(&name));

            if root.exists() {
                trace!(target: LOG_TARGET, "Found at {}", color::path(&root));

                return Template::new(name, root);
            }
        }

        Err(GeneratorError::MissingTemplate(name))
    }

    pub async fn generate(&self, template: &Template) -> Result<(), GeneratorError> {
        let mut futures = vec![];

        debug!(
            target: LOG_TARGET,
            "Generating template {} files",
            color::id(&template.name),
        );

        for file in &template.files {
            if file.should_write() {
                futures.push(template.write_file(file));
            }
        }

        try_join_all(futures).await?;

        debug!(target: LOG_TARGET, "Generation complete!");

        Ok(())
    }
}
