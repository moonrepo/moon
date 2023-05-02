use moon_config::RustConfig;
use moon_platform_runtime::Version;
use moon_tool::{Tool, ToolError};
use proto::{async_trait, Proto};
use std::path::PathBuf;

#[derive(Debug)]
pub struct RustTool {
    pub config: RustConfig,

    pub global: bool,
}

impl RustTool {
    pub fn new(
        _proto: &Proto,
        config: &RustConfig,
        version: &Version,
    ) -> Result<RustTool, ToolError> {
        let mut deno = RustTool {
            config: config.to_owned(),
            global: true,
        };

        if version.is_global() {
            deno.global = true;
            // node.config.version = None;
        } else {
            // node.config.version = Some(version.number.to_owned());
        };

        Ok(deno)
    }
}

#[async_trait]
impl Tool for RustTool {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn get_bin_path(&self) -> Result<PathBuf, ToolError> {
        Ok(PathBuf::from("rust"))
    }
}
