use proto_error::ProtoError;
use std::{fs, path::Path};

#[async_trait::async_trait]
pub trait Detector<'tool>: Send + Sync {
    /// Attempt to detect an applicable version from the provided working directory.
    async fn detect_version_from(&self, _working_dir: &Path) -> Result<Option<String>, ProtoError> {
        Ok(None)
    }
}

pub fn load_version_file(path: &Path) -> Result<String, ProtoError> {
    Ok(fs::read_to_string(path)
        .map_err(|e| ProtoError::Fs(path.to_path_buf(), e.to_string()))?
        .trim()
        .to_owned())
}
