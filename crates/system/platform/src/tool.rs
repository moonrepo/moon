use moon_tool::Tool;
use std::path::PathBuf;

#[derive(Debug, Default)]
pub struct SystemToolStub {
    bin_path: PathBuf,
}

impl Tool for SystemToolStub {
    fn as_any(&self) -> &(dyn std::any::Any + Send + Sync) {
        self
    }

    fn get_bin_path(&self) -> miette::Result<PathBuf> {
        Ok(self.bin_path.to_path_buf())
    }
}
