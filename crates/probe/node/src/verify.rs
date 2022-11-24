use crate::NodeLanguage;
use log::debug;
use probe_core::{
    async_trait, download_from_url, get_sha256_hash_of_file, ProbeError, Resolvable, Verifiable,
};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

#[async_trait]
impl Verifiable<'_> for NodeLanguage {
    fn get_checksum_path(&self) -> Result<PathBuf, ProbeError> {
        Ok(self
            .temp_dir
            .join(format!("{}-SHASUMS256.txt", self.get_resolved_version())))
    }

    async fn download_checksum(
        &self,
        to_file: &Path,
        from_url: Option<&str>,
    ) -> Result<(), ProbeError> {
        if to_file.exists() {
            debug!(target: "probe:node:verify", "Already downloaded checksum, continuing");

            return Ok(());
        }

        let version = self.get_resolved_version();
        let from_url = match from_url {
            Some(url) => url.to_owned(),
            None => format!("https://nodejs.org/dist/v{}/SHASUMS256.txt", version),
        };

        debug!(target: "probe:node:verify", "Attempting to download from {}", from_url);

        download_from_url(&from_url, &to_file).await?;

        debug!(target: "probe:node:verify", "Successfully downloaded");

        Ok(())
    }

    async fn verify_checksum(
        &self,
        checksum_file: &Path,
        download_file: &Path,
    ) -> Result<bool, ProbeError> {
        debug!(
            target: "probe:node:verify",
            "Verifiying checksum of downloaded file {} using {}",
            download_file.to_string_lossy(),
            checksum_file.to_string_lossy(),
        );

        let checksum = get_sha256_hash_of_file(download_file)?;

        let file = File::open(checksum_file)
            .map_err(|e| ProbeError::Fs(checksum_file.to_path_buf(), e.to_string()))?;
        let file_name = download_file
            .file_name()
            .unwrap_or_default()
            .to_str()
            .unwrap_or_default();

        for line in BufReader::new(file).lines().flatten() {
            // <checksum>  node-v<version>-<os>-<arch>.tar.gz
            if line.starts_with(&checksum) && line.ends_with(file_name) {
                debug!(target: "probe:node:verify", "Successfully verified, checksum matches");

                return Ok(true);
            }
        }

        Err(ProbeError::InvalidChecksum(
            download_file.to_path_buf(),
            checksum_file.to_path_buf(),
        ))
    }
}
