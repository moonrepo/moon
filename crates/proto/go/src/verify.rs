use crate::download::get_archive_file;
use crate::GoLanguage;
use log::debug;
use proto_core::{
    async_trait, color, get_sha256_hash_of_file, Describable, ProtoError, Resolvable, Verifiable,
};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

#[async_trait]
impl Verifiable<'_> for GoLanguage {
    fn get_checksum_path(&self) -> Result<PathBuf, ProtoError> {
        Ok(self
            .temp_dir
            .join(format!("{}-SHASUMS256.txt", self.get_resolved_version())))
    }

    fn get_checksum_url(&self) -> Result<String, ProtoError> {
        Ok(format!(
            "https://dl.google.com/go/{}.sha256",
            get_archive_file(self.get_resolved_version())?
        ))
    }

    async fn verify_checksum(
        &self,
        checksum_file: &Path,
        download_file: &Path,
    ) -> Result<bool, ProtoError> {
        debug!(
            target: self.get_log_target(),
            "Verifiying checksum of downloaded file {} using {}",
            color::path(download_file),
            color::path(checksum_file),
        );

        let checksum = get_sha256_hash_of_file(download_file)?;

        let file = File::open(checksum_file)
            .map_err(|e| ProtoError::Fs(checksum_file.to_path_buf(), e.to_string()))?;

        for line in BufReader::new(file).lines().flatten() {
            if line.starts_with(&checksum) {
                debug!(target: self.get_log_target(), "Successfully verified, checksum matches");

                return Ok(true);
            }
        }

        Err(ProtoError::VerifyInvalidChecksum(
            download_file.to_path_buf(),
            checksum_file.to_path_buf(),
        ))
    }
}
