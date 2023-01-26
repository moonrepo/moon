use proto::{ProtoError, ToolType};
use rustc_hash::FxHashMap;
use std::{fs, path::Path};
use toml::Value;

pub const CONFIG_NAME: &str = ".prototools";

#[derive(Debug)]
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
}
