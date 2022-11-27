use crate::errors::ProbeError;
use std::path::Path;

#[async_trait::async_trait]
pub trait Executable<'tool>: Send + Sync {
    /// Find the absolute file path to the tool's binary that will be executed.
    /// This happens after a tool has been downloaded and installed.
    async fn find_bin_path(&mut self) -> Result<(), ProbeError> {
        Ok(())
    }

    /// Returns an absolute file path to the executable binary for the tool.
    fn get_bin_path(&self) -> Result<&Path, ProbeError>;
}
