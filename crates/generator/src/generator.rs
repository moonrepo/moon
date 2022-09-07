use crate::errors::GeneratorError;
use crate::template::{Template, TemplateFile};
use futures::stream::{FuturesUnordered, StreamExt};
use moon_config::{load_template_config_template, GeneratorConfig};
use moon_constants::CONFIG_TEMPLATE_FILENAME;
use moon_utils::{fs, regex::clean_id};
use std::path::{Path, PathBuf};

pub struct Generator {
    config: GeneratorConfig,

    workspace_root: PathBuf,
}

impl Generator {
    pub fn create(workspace_root: &Path, config: &GeneratorConfig) -> Result<Self, GeneratorError> {
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

        for template_path in &self.config.templates {
            let root = self.workspace_root.join(template_path).join(&name);

            if root.exists() {
                return Template::new(name, root);
            }
        }

        Err(GeneratorError::MissingTemplate(name))
    }

    pub async fn generate(&self, files: &[TemplateFile]) -> Result<(), GeneratorError> {
        let mut futures = FuturesUnordered::new();

        for file in files {
            if file.should_write() {
                futures.push(file.generate());
            }
        }

        // Copy all the files in parallel
        loop {
            match futures.next().await {
                Some(Err(e)) => return Err(GeneratorError::Moon(e)),
                Some(Ok(_)) => {}
                None => break,
            }
        }

        Ok(())
    }
}
