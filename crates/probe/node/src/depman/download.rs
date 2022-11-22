use crate::depman::NodeDependencyManager;
use log::debug;
use probe_core::{async_trait, download_from_url, Downloadable, ProbeError, Resolvable};
use std::path::{Path, PathBuf};

#[async_trait]
impl<'tool> Downloadable<'tool> for NodeDependencyManager<'tool> {
    fn get_download_path(&self) -> Result<PathBuf, ProbeError> {
        Ok(self
            .temp_dir
            .join(format!("{}.tgz", self.get_resolved_version())))
    }

    async fn download(&self, to_file: &Path, from_url: Option<&str>) -> Result<(), ProbeError> {
        if to_file.exists() {
            debug!(target: "probe:node:download", "Already downloaded, continuing");

            return Ok(());
        }

        let from_url = from_url.unwrap_or(&self.get_dist().tarball);

        debug!(target: "probe:node:download", "Attempting to download from {}", from_url);

        download_from_url(&from_url, &to_file).await?;

        debug!(target: "probe:node:download", "Successfully downloaded");

        Ok(())
    }
}
