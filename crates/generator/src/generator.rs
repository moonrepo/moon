use moon_config::GeneratorConfig;
use std::path::{Path, PathBuf};

use crate::GeneratorError;

pub struct Generator {
    config: GeneratorConfig,

    workspace_root: PathBuf,
}

impl Generator {
    pub fn new(workspace_root: &Path, config: &GeneratorConfig) -> Self {
        Generator {
            config: config.to_owned(),
            workspace_root: workspace_root.to_path_buf(),
        }
    }

    pub fn generate(&self, name: &str) -> Result<(), GeneratorError> {
        let _template_root = self.find_template_root(name);

        Ok(())
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
