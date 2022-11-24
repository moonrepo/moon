use crate::depman::NodeDependencyManager;
use log::debug;
use probe_core::{
    async_trait, download_from_url, get_sha256_hash_of_file, Describable, ProbeError, Resolvable,
    Verifiable,
};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

#[async_trait]
impl Verifiable<'_> for NodeDependencyManager {
    fn get_checksum_path(&self) -> Result<PathBuf, ProbeError> {
        Ok(self
            .temp_dir
            .join(format!("{}-SHASUMS256.txt", self.get_resolved_version())))
    }

    async fn download_checksum(
        &self,
        to_file: &Path,
        from_url: Option<&str>,
    ) -> Result<bool, ProbeError> {
        Ok(true)
    }

    async fn verify_checksum(
        &self,
        checksum_file: &Path,
        download_file: &Path,
    ) -> Result<bool, ProbeError> {
        Ok(true)
    }
}
