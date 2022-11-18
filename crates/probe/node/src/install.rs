use crate::download::get_archive_file_path;
use crate::NodeLanguage;
use log::debug;
use probe_core::{async_trait, untar, unzip, Installable, ProbeError, Resolvable};
use std::path::{Path, PathBuf};

#[async_trait]
impl<'tool> Installable<'tool> for NodeLanguage<'tool> {
    fn get_install_dir(&self) -> Result<PathBuf, ProbeError> {
        Ok(self.install_dir.join(self.get_resolved_version()))
    }

    async fn install(&self, install_dir: &Path, download_path: &Path) -> Result<(), ProbeError> {
        if install_dir.exists() {
            debug!(target: "probe:node:install", "Already installed, continuing");

            return Ok(());
        }

        let prefix = get_archive_file_path(self.get_resolved_version())?;

        debug!(
            target: "probe:node:install",
            "Attempting to install from {} to {}",
            download_path.to_string_lossy(),
            install_dir.to_string_lossy(),
        );

        if download_path.extension().unwrap_or_default() == "zip" {
            unzip(download_path, install_dir, Some(&prefix))?;
        } else {
            untar(download_path, install_dir, Some(&prefix))?;
        }

        debug!(target: "probe:node:install", "Successfully installed");

        Ok(())
    }
}
