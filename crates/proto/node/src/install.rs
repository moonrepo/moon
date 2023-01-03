use crate::download::get_archive_file_path;
use crate::NodeLanguage;
use log::debug;
use proto_core::{async_trait, untar, unzip, Describable, Installable, ProtoError, Resolvable};
use std::path::{Path, PathBuf};

#[async_trait]
impl Installable<'_> for NodeLanguage {
    fn get_install_dir(&self) -> Result<PathBuf, ProtoError> {
        Ok(self.base_dir.join(self.get_resolved_version()))
    }

    async fn install(&self, install_dir: &Path, download_path: &Path) -> Result<bool, ProtoError> {
        if install_dir.exists() {
            debug!(target: self.get_log_target(), "Tool already installed, continuing");

            return Ok(false);
        }

        if !download_path.exists() {
            return Err(ProtoError::InstallMissingDownload(self.get_name()));
        }

        let prefix = get_archive_file_path(self.get_resolved_version())?;

        debug!(
            target: self.get_log_target(),
            "Attempting to install {} to {}",
            download_path.to_string_lossy(),
            install_dir.to_string_lossy(),
        );

        if download_path.extension().unwrap_or_default() == "zip" {
            unzip(download_path, install_dir, Some(&prefix))?;
        } else {
            untar(download_path, install_dir, Some(&prefix))?;
        }

        debug!(target: self.get_log_target(), "Successfully installed tool");

        Ok(true)
    }
}
