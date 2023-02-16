use moon_config::DenoConfig;
use moon_tool::{Tool, ToolError};
use proto::{async_trait, Proto};
use rustc_hash::FxHashMap;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct DenoTool {
    pub config: DenoConfig,

    pub global: bool,

    pub temp: PathBuf,
}

impl DenoTool {
    pub fn new(_proto: &Proto, config: &DenoConfig, _version: &str) -> Result<DenoTool, ToolError> {
        let deno = DenoTool {
            config: config.to_owned(),
            global: true,
            temp: PathBuf::from("deno"),
        };

        Ok(deno)
    }
}

#[async_trait]
impl Tool for DenoTool {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn get_bin_path(&self) -> Result<&Path, ToolError> {
        Ok(&self.temp)
    }

    fn get_shim_path(&self) -> Option<&Path> {
        None
    }

    fn get_version(&self) -> &str {
        ""
    }

    async fn setup(
        &mut self,
        _last_versions: &mut FxHashMap<String, String>,
    ) -> Result<u8, ToolError> {
        Ok(0)
    }

    async fn teardown(&mut self) -> Result<(), ToolError> {
        Ok(())
    }
}
