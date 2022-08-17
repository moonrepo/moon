use crate::errors::ConfigError;
use moon_error::map_io_to_fs_error;
use moon_utils::path;
use std::collections::VecDeque;
use std::fs;
use std::path::{Path, PathBuf};
use yaml_rust::{Yaml, YamlLoader};

pub fn gather_extended_sources<T: AsRef<Path>>(base_config: T) -> Result<Vec<String>, ConfigError> {
    let base_config = base_config.as_ref();
    let mut queue = VecDeque::from(vec![path::to_string(base_config)?]);
    let mut sources = vec![];

    while !queue.is_empty() {
        if let Some(config) = queue.pop_front() {
            if !config.ends_with(".yml") && !config.ends_with(".yaml") {
                return Err(ConfigError::UnsupportedExtendsDocument(config));
            }

            // We cant parse URLs, so end immediately
            if config.starts_with("https") {
                sources.push(config);
                break;
            } else if config.starts_with("http") {
                return Err(ConfigError::UnsupportedHttps(config));
            }

            // Otherwise we have a possible file path
            let config_path = PathBuf::from(&config);

            if !config_path.exists() {
                return Err(ConfigError::MissingFile(config));
            }

            sources.push(config);

            // Parse the YAML document and attempt to extract the `extends` field.
            // We can't use serde here as the shape of the document may be invalid
            // or incomplete.
            let yaml = YamlLoader::load_from_str(
                &fs::read_to_string(&config_path)
                    .map_err(|e| map_io_to_fs_error(e, config_path.to_owned()))?,
            )
            .map_err(|e| ConfigError::InvalidYaml(config_path.clone(), e.to_string()))?;

            let doc = &yaml[0];

            // Field does not exist!
            if doc["extends"].is_badvalue() {
                continue;
            }

            match &doc["extends"] {
                Yaml::String(extends) => {
                    if extends.starts_with("http") {
                        queue.push_back(extends.to_owned());
                    } else {
                        queue.push_back(path::to_string(
                            config_path.parent().unwrap().join(extends),
                        )?);
                    }
                }
                _ => {
                    return Err(ConfigError::InvalidExtendsField);
                }
            }
        }
    }

    // Reverse the order as we must load the extended leaf first,
    // with the initial base config overriding as last.
    sources.reverse();

    Ok(sources)
}
