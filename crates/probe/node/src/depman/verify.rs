use crate::depman::NodeDependencyManager;
use probe_core::{async_trait, ProbeError, Resolvable, Verifiable};
use std::path::{Path, PathBuf};

// TODO: implement PGP/ECDSA signature verify
// https://docs.npmjs.com/about-registry-signatures
#[async_trait]
impl Verifiable<'_> for NodeDependencyManager {
    fn get_checksum_path(&self) -> Result<PathBuf, ProbeError> {
        Ok(self
            .temp_dir
            .join(format!("{}.pub", self.get_resolved_version())))
    }

    async fn download_checksum(
        &self,
        _to_file: &Path,
        _from_url: Option<&str>,
    ) -> Result<bool, ProbeError> {
        Ok(true)
    }

    async fn verify_checksum(
        &self,
        _checksum_file: &Path,
        _download_file: &Path,
    ) -> Result<bool, ProbeError> {
        Ok(true)
    }
}
