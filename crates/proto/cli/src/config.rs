use clap::ValueEnum;
use proto::{ProtoError, ToolType};
use rustc_hash::FxHashMap;
use std::{fs, path::Path};
use toml::{map::Map, Value};

pub const CONFIG_NAME: &str = ".prototools";

#[derive(Debug, Default)]
pub struct Config {
    pub tools: FxHashMap<ToolType, String>,
}

impl Config {
    pub fn load(path: &Path) -> Result<Self, ProtoError> {
        let contents = fs::read_to_string(path)
            .map_err(|e| ProtoError::Fs(path.to_path_buf(), e.to_string()))?;

        let config = contents
            .parse::<Value>()
            .map_err(|e| ProtoError::InvalidConfig(path.to_path_buf(), e.to_string()))?;

        let mut tools = FxHashMap::default();

        if let Value::Table(table) = config {
            for (key, value) in table {
                let tool_type = key.parse::<ToolType>()?;

                if let Value::String(version) = value {
                    tools.insert(tool_type, version);
                } else {
                    return Err(ProtoError::InvalidConfig(
                        path.to_path_buf(),
                        format!("Expected a version string for \"{key}\"."),
                    ));
                }
            }
        } else {
            return Err(ProtoError::InvalidConfig(
                path.to_path_buf(),
                "Expected a mapping of tools to versions.".into(),
            ));
        }

        Ok(Config { tools })
    }

    pub fn save(&self, path: &Path) -> Result<(), ProtoError> {
        let mut map = Map::with_capacity(self.tools.len());

        for (tool, version) in &self.tools {
            map.insert(
                tool.to_possible_value().unwrap().get_name().to_owned(),
                Value::String(version.to_owned()),
            );
        }

        let data = toml::to_string_pretty(&Value::Table(map))
            .map_err(|e| ProtoError::Toml(path.to_path_buf(), e.to_string()))?;

        fs::write(path, data).map_err(|e| ProtoError::Fs(path.to_path_buf(), e.to_string()))?;

        Ok(())
    }
}
