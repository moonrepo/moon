use crate::errors::ConfigError;
use moon_utils::{
    fs::temp,
    path, time,
    yaml::{self, YamlValue},
};
use std::collections::VecDeque;
use std::path::{Path, PathBuf};

pub fn download_and_cache_config(url: &str) -> Result<PathBuf, ConfigError> {
    let file = temp::get_file(url, "yml");

    if temp::read(&file)?.is_some() {
        return Ok(file);
    }

    let error_handler = |_| ConfigError::FailedDownload(url.to_owned());
    let data = reqwest::blocking::get(url)
        .map_err(error_handler)?
        .text()
        .map_err(error_handler)?;

    temp::write(
        &file,
        format!(
            "# source: {}\n# timestamp: {}\n\n{}",
            url,
            time::now_timestamp(),
            data
        ),
    )?;

    Ok(file)
}

pub fn gather_extended_sources<T: AsRef<Path>>(
    base_config: T,
) -> Result<Vec<PathBuf>, ConfigError> {
    let base_config = base_config.as_ref();
    let mut queue = VecDeque::from(vec![path::to_string(base_config)?]);
    let mut sources = vec![];

    while !queue.is_empty() {
        if let Some(config) = queue.pop_front() {
            let mut config_path = PathBuf::from(&config);

            if !config.ends_with(".yml") && !config.ends_with(".yaml") {
                return Err(ConfigError::UnsupportedExtendsDocument(config));
            }

            // For https, download the config and store it in the temp cache
            if config.starts_with("https") || config.contains("127.0.0.1") {
                config_path = download_and_cache_config(&config)?;
            } else if config.starts_with("http") {
                return Err(ConfigError::UnsupportedHttps(config));
            }

            // Otherwise we have a possible file path
            if !config_path.exists() {
                return Err(ConfigError::MissingFile(config));
            }

            sources.push(config_path.clone());

            // Parse the YAML document and attempt to extract the `extends` field.
            // We can't use serde here as the shape of the document may be invalid
            // or incomplete.
            let doc: YamlValue = yaml::read(&config_path)
                .map_err(|e| ConfigError::InvalidYaml(config_path.clone(), e.to_string()))?;

            if let Some(extends_field) = doc.get("extends") {
                match &extends_field {
                    YamlValue::String(extends) => {
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
    }

    // Reverse the order as we must load the extended leaf first,
    // with the initial base config overriding as last.
    sources.reverse();

    Ok(sources)
}
