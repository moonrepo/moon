use moon_tool::Tool;
use std::path::PathBuf;

#[derive(Debug, Default)]
pub struct SystemToolStub {
    _bin_path: PathBuf,
}

impl Tool for SystemToolStub {
    fn as_any(&self) -> &(dyn std::any::Any + Send + Sync) {
        self
    }
}
