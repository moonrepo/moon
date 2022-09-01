use crate::errors::GeneratorError;
use crate::template::Template;
use moon_config::{load_template_config_template, GeneratorConfig};
use moon_constants::CONFIG_TEMPLATE_FILENAME;
use moon_utils::{fs, regex::clean_id};
use std::path::{Path, PathBuf};

pub struct Generator {
    pub config: GeneratorConfig,

    workspace_root: PathBuf,
}

impl Generator {
    pub fn create(workspace_root: &Path, config: &GeneratorConfig) -> Result<Self, GeneratorError> {
        Ok(Generator {
            config: config.to_owned(),
            workspace_root: workspace_root.to_path_buf(),
        })
    }

    pub fn generate(&self, name: &str) -> Result<(), GeneratorError> {
        let template_name = clean_id(name);
        let _template_root = self.find_template_root(&template_name);

        Ok(())
    }

    pub async fn generate_template(&self, name: &str) -> Result<Template, GeneratorError> {
        let template_name = clean_id(name);
        let template_root = self
            .workspace_root
            .join(&self.config.templates[0])
            .join(&template_name);

        if template_root.exists() {
            return Err(GeneratorError::ExistingTemplate(
                template_name,
                template_root,
            ));
        }

        fs::create_dir_all(&template_root).await?;

        fs::write(
            template_root.join(CONFIG_TEMPLATE_FILENAME),
            load_template_config_template(),
        )
        .await?;

        Ok(Template {
            name: template_name,
            root: template_root,
        })
    }

    fn find_template_root(&self, name: &str) -> Result<PathBuf, GeneratorError> {
        for template_path in &self.config.templates {
            let template_root = self.workspace_root.join(template_path).join(name);

            if template_root.exists() {
                return Ok(template_root);
            }
        }

        Err(GeneratorError::MissingTemplate(name.to_owned()))
    }
}
