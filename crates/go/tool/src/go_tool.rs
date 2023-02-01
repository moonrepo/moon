use moon_tool::{Tool, ToolError};
use std::path::Path;
use proto::{
    async_trait, go::GoLanguage, Describable, Executable, Installable, Proto, Resolvable,
    Shimable, Tool as ProtoTool,
};

#[derive(Debug)]
pub struct GoTool {
    pub tool: GoLanguage,
}

impl GoTool {
    pub fn new(proto: &Proto) -> Result<GoTool, ToolError> {
        Ok(GoTool {
            tool: GoLanguage::new(proto),
        })
    }
}

impl Tool for GoTool {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn get_bin_path(&self) -> Result<&Path, ToolError> {
        Ok(self.tool.get_bin_path()?)
    }

    fn get_version(&self) -> &str {
        "latest"
    }
}
