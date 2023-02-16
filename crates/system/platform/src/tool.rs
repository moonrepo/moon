use moon_tool::{Tool, ToolError};
use std::path::PathBuf;

#[derive(Debug, Default)]
pub struct SystemToolStub {
    bin_path: PathBuf,
}

impl Tool for SystemToolStub {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn get_bin_path(&self) -> Result<PathBuf, ToolError> {
        Ok(self.bin_path.to_path_buf())
    }
}
