use crate::GeneratorError;
use moon_config::{format_error_line, format_figment_errors, ConfigError, TemplateConfig};
use moon_constants::CONFIG_TEMPLATE_FILENAME;
use moon_utils::fs;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct TemplateFile {
    pub dest_path: PathBuf, // absolute path to destination
    pub existed: bool,
    pub overwrite: bool,
    pub path: PathBuf,        // relative
    pub source_path: PathBuf, // absolute path to source (in templates dir)
}

#[derive(Debug, Eq, PartialEq)]
pub struct Template {
    pub config: TemplateConfig,
    pub name: String,
    pub root: PathBuf,
}

impl Template {
    pub fn new(name: String, root: PathBuf) -> Result<Template, GeneratorError> {
        let config = TemplateConfig::load(root.join(CONFIG_TEMPLATE_FILENAME)).unwrap();
        //     Ok(cfg) => Ok(cfg),
        //     Err(errors) => {
        //         return Err(GeneratorError::InvalidConfigFile(
        //             if let ConfigError::FailedValidation(valids) = errors {
        //                 format_figment_errors(valids)
        //             } else {
        //                 format_error_line(errors.to_string())
        //             },
        //         ))
        //     }
        // };

        Ok(Template { config, name, root })
    }

    pub async fn get_template_files(
        &self,
        dest: &Path,
    ) -> Result<Vec<TemplateFile>, GeneratorError> {
        let mut files = vec![];

        for entry in fs::read_dir_all(&self.root).await? {
            // This is moons schema, so skip it
            if entry.file_name() == CONFIG_TEMPLATE_FILENAME {
                continue;
            }

            let source_path = entry.path();
            let path = source_path.strip_prefix(&self.root).unwrap();
            let dest_path = dest.join(path);
            let existed = dest_path.exists();

            files.push(TemplateFile {
                dest_path,
                existed,
                overwrite: false,
                path: path.to_path_buf(),
                source_path,
            })
        }

        Ok(files)
    }
}
