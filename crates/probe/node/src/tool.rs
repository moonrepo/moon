use probe_core::{Downloadable, Probe, ProbeError, Verifiable};

pub struct NodeLanguage<'tool> {
    pub version: &'tool str,
}

impl<'tool> NodeLanguage<'tool> {
    pub async fn setup(&self, parent: &Probe) -> Result<(), ProbeError> {
        // Download the archive
        let download_file = self.download(parent, None).await?;

        // Verify the archive
        self.download_checksum(parent, None).await?;
        self.verify_checksum(parent, &download_file).await?;

        // TODO install

        Ok(())
    }
}
