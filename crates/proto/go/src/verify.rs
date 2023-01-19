use crate::GoLanguage;
use crate::download::get_archive_file;
use log::debug;
use proto_core::{
    async_trait, color, download_from_url, get_sha256_hash_of_file, Describable, ProtoError,
    Resolvable, Verifiable,
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

    async fn download_checksum(
        &self,
        to_file: &Path,
        from_url: Option<&str>,
    ) -> Result<bool, ProtoError> {
        if to_file.exists() {
            debug!(target: self.get_log_target(), "Checksum already downloaded, continuing");

            return Ok(false);
        }

        let version = self.get_resolved_version();
        let download_url = get_archive_file(version)?;
        let from_url = match from_url {
            Some(url) => url.to_owned(),
            None => format!("https://dl.google.com/go/{}.sha256", download_url),
        };

        debug!(target: self.get_log_target(), "Attempting to download checksum from {}", color::url(&from_url));

        download_from_url(&from_url, &to_file).await?;

        debug!(target: self.get_log_target(), "Successfully downloaded checksum");

        Ok(true)
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
