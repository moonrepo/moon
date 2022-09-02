use crate::errors::GeneratorError;
use crate::template::Template;
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

    pub async fn generate(&self, name: &str) -> Result<Template, GeneratorError> {
        let name = clean_id(name);
        let root = self.find_template_root(&name)?;
        // let files = fs::read_dir_all(&root).await?;

        // dbg!(&name);
        // dbg!(&root);
        // dbg!(&files);
        // dbg!(dest.as_ref());

        Ok(Template::new(name, root)?)
    }

    /// Generate a new template, with schema, into the first configured template path.
    /// Will error if a template of the same name already exists.
    pub async fn generate_template(&self, name: &str) -> Result<Template, GeneratorError> {
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

        Ok(Template::new(name, root)?)
    }

    /// Find a template with the provided name amongst the list of possible template paths.
    fn find_template_root(&self, id: &str) -> Result<PathBuf, GeneratorError> {
        for template_path in &self.config.templates {
            let template_root = self.workspace_root.join(template_path).join(id);

            if template_root.exists() {
                return Ok(template_root);
            }
        }

        Err(GeneratorError::MissingTemplate(id.to_owned()))
    }
}
