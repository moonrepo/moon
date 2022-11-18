use crate::download::get_archive_file_path;
use crate::tool::NodeLanguage;
use probe_core::{async_trait, untar, unzip, Installable, ProbeError, Resolvable};
use std::path::{Path, PathBuf};

#[async_trait]
impl<'tool> Installable<'tool> for NodeLanguage<'tool> {
    fn get_install_dir(&self, tools_dir: &Path) -> Result<PathBuf, ProbeError> {
        Ok(tools_dir.join("node").join(self.get_resolved_version()))
    }

    async fn install(&self, install_dir: &Path, download_path: &Path) -> Result<(), ProbeError> {
        let prefix = get_archive_file_path(self.get_resolved_version())?;

        if download_path.extension().unwrap_or_default() == "zip" {
            unzip(download_path, install_dir, Some(&prefix))?;
        } else {
            untar(download_path, install_dir, Some(&prefix))?;
        }

        Ok(())
    }
}
