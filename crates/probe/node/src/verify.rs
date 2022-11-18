use crate::tool::NodeLanguage;
use probe_core::{
    async_trait, download_from_url, get_sha256_hash_of_file, Probe, ProbeError, Resolvable,
    Verifiable,
};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

#[async_trait]
impl<'tool> Verifiable<'tool, Probe> for NodeLanguage<'tool> {
    fn get_checksum_path(&self, parent: &Probe) -> Result<PathBuf, ProbeError> {
        Ok(parent.temp_dir.join("node").join("SHASUMS256.txt"))
    }

    async fn download_checksum(
        &self,
        parent: &Probe,
        checksum_url: Option<&str>,
    ) -> Result<(), ProbeError> {
        let checksum_file = self.get_checksum_path(parent)?;

        if checksum_file.exists() {
            return Ok(());
        }

        let version = self.get_resolved_version();
        let checksum_url = match checksum_url {
            Some(url) => url.to_owned(),
            None => format!("https://nodejs.org/dist/v{}/SHASUMS256.txt", version),
        };

        download_from_url(&checksum_url, &checksum_file).await?;

        Ok(())
    }

    async fn verify_checksum(
        &self,
        parent: &Probe,
        download_file: &Path,
    ) -> Result<bool, ProbeError> {
        let checksum_file = self.get_checksum_path(parent)?;
        let checksum = get_sha256_hash_of_file(download_file)?;

        let file = File::open(&checksum_file)
            .map_err(|e| ProbeError::FileSystem(checksum_file.to_path_buf(), e.to_string()))?;
        let file_name = download_file
            .file_name()
            .unwrap_or_default()
            .to_str()
            .unwrap_or_default();

        for line in BufReader::new(file).lines().flatten() {
            // <checksum>  node-v<version>-<os>-<arch>.tar.gz
            if line.starts_with(&checksum) && line.ends_with(file_name) {
                return Ok(true);
            }
        }

        Err(ProbeError::InvalidChecksum(
            download_file.to_path_buf(),
            checksum_file,
        ))
    }
}
