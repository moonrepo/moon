use probe_core::{Downloadable, Probe, ProbeError, Verifiable};

pub struct NodeLanguage<'tool> {
    pub version: &'tool str,
}

impl<'tool> NodeLanguage<'tool> {
    pub async fn setup(&self, parent: &Probe) -> Result<(), ProbeError> {
        // Download the archive
        let download_path = self.get_download_path(&parent.temp_dir)?;

        self.download(&download_path, None).await?;

        // Verify the archive
        let checksum_path = self.get_checksum_path(&parent.temp_dir)?;

        self.download_checksum(&checksum_path, None).await?;
        self.verify_checksum(&checksum_path, &download_path).await?;

        // TODO install

        Ok(())
    }
}
